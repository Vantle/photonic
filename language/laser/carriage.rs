use super::pool::Demand;
use super::trace::Trace;
use super::{CHUNK, Laser, Progress, map};
use crate::executor::Executor;
use crate::profile;
use std::collections::BTreeMap;
use std::num::NonZeroU32;
use std::ops::Range;

// A round can carry millions of traces. Carrying and inserting them a batch at a time keeps only
// one batch of new traces waiting, and every trace a batch inserts is known to the batches after
// it, so the traces, their positions and their landings are the ones a single batch would give.
const BATCH: usize = 1 << 20;

struct Crossing {
    event: usize,
    range: Range<usize>,
}

// Where each trace of a crossing lands: nowhere when its frame did not exist before the event,
// otherwise at a trace the source already had or at one of the new traces the crossing carried.
#[derive(Clone, Copy)]
enum Landing {
    Lost,
    Known(usize),
    New(usize),
}

// Each new trace remembers the position it was carried from, which is where its deduction goes on.
struct Carried {
    event: usize,
    source: usize,
    count: usize,
    landing: Vec<Landing>,
    list: Vec<(Trace, usize)>,
}

fn batch(crossing: Vec<Crossing>) -> Vec<Vec<Crossing>> {
    let mut batch = Vec::new();
    let mut current = Vec::new();
    let mut size = 0;
    for crossing in crossing {
        let mut start = crossing.range.start;
        while start < crossing.range.end {
            let end = crossing.range.end.min(start + BATCH - size);
            current.push(Crossing {
                event: crossing.event,
                range: start..end,
            });
            size += end - start;
            start = end;
            if size == BATCH {
                batch.push(std::mem::take(&mut current));
                size = 0;
            }
        }
    }
    if !current.is_empty() {
        batch.push(current);
    }
    batch
}

fn land(position: usize) -> NonZeroU32 {
    NonZeroU32::new(u32::try_from(position + 1).expect("fewer than 2^32 traces"))
        .expect("a position after zero")
}

fn settle(landing: Vec<Landing>, position: &[usize]) -> Vec<Option<NonZeroU32>> {
    landing
        .into_iter()
        .map(|landing| match landing {
            Landing::Lost => None,
            Landing::Known(found) => Some(land(found)),
            Landing::New(index) => Some(land(position[index])),
        })
        .collect()
}

impl Laser {
    // Where a trace of an event's target landed at its source, once the event carried it.
    pub(super) fn landing(&self, event: usize, position: usize) -> Option<usize> {
        let found = self.crossed[event].get(position).copied().flatten()?;
        Some(found.get() as usize - 1)
    }

    pub(super) fn propagate(
        &mut self,
        executor: Option<&Executor>,
        changed: Vec<usize>,
    ) -> (usize, Vec<(usize, Range<usize>)>) {
        let mut count = 0;
        let mut grown = BTreeMap::<usize, Range<usize>>::new();
        for crossing in batch(self.plan(changed)) {
            let demand = self.survey(executor, &crossing);
            self.prepare(executor, demand);
            let carried = self.carry(executor, crossing);
            let (value, range) = self.insert(executor, carried);
            self.capture.forget(executor);
            count += value;
            for (index, range) in range {
                grown
                    .entry(index)
                    .and_modify(|known| known.end = range.end)
                    .or_insert(range);
            }
        }
        (count, grown.into_iter().collect())
    }

    fn plan(&mut self, mut changed: Vec<usize>) -> Vec<Crossing> {
        let _scope = profile::Scope::new(profile::Phase::Planning);
        changed.sort_unstable();
        changed.dedup();
        let mut crossing = Vec::new();
        for index in changed {
            let before = self.progress[index];
            let after = Progress {
                trace: self.trace[index].len(),
                event: self.incoming[index].len(),
            };
            for (position, &event) in self.incoming[index].iter().enumerate() {
                let start = if position < before.event {
                    before.trace
                } else {
                    0
                };
                if start < after.trace {
                    crossing.push(Crossing {
                        event,
                        range: start..after.trace,
                    });
                }
            }
            self.progress[index] = after;
        }
        crossing
    }

    fn survey(&self, executor: Option<&Executor>, crossing: &[Crossing]) -> Vec<(usize, Demand)> {
        let _scope = profile::Scope::new(profile::Phase::Planning);
        map(executor, crossing.iter().collect(), |crossing| {
            self.need(crossing)
        })
    }

    fn need(&self, crossing: &Crossing) -> (usize, Demand) {
        let event = &self.event[crossing.event];
        let set = &self.trace[event.target];
        let mut demand = Demand::default();
        for position in crossing.range.clone() {
            set[position].need(
                crossing.event,
                &self.passage[crossing.event],
                &self.pool,
                &mut demand,
            );
        }
        (crossing.event, demand)
    }

    fn prepare(&mut self, executor: Option<&Executor>, demand: Vec<(usize, Demand)>) {
        let _scope = profile::Scope::new(profile::Phase::Imaging);
        self.pool.prepare(executor, demand, &self.passage);
    }

    fn carry(&self, executor: Option<&Executor>, crossing: Vec<Crossing>) -> Vec<Carried> {
        let _scope = profile::Scope::new(profile::Phase::Carriage);
        let chunk = crossing
            .into_iter()
            .flat_map(|crossing| {
                let end = crossing.range.end;
                crossing.range.step_by(CHUNK).map(move |first| Crossing {
                    event: crossing.event,
                    range: first..(first + CHUNK).min(end),
                })
            })
            .collect();
        map(executor, chunk, |crossing| self.cross(crossing))
    }

    fn cross(&self, crossing: Crossing) -> Carried {
        let event = &self.event[crossing.event];
        let set = &self.trace[event.target];
        let known = &self.trace[event.source];
        let passage = &self.passage[crossing.event];
        let mut count = 0;
        let mut list = Vec::new();
        let landing = crossing
            .range
            .map(|position| {
                let Some(carried) =
                    set[position].carry(crossing.event, passage, &self.pool, &self.capture)
                else {
                    return Landing::Lost;
                };
                count += 1;
                if let Some(found) = known.get_index_of(&carried) {
                    return Landing::Known(found);
                }
                list.push((carried, position));
                Landing::New(list.len() - 1)
            })
            .collect();
        Carried {
            event: crossing.event,
            source: event.source,
            count,
            landing,
            list,
        }
    }

    fn insert(
        &mut self,
        executor: Option<&Executor>,
        carried: Vec<Carried>,
    ) -> (usize, Vec<(usize, Range<usize>)>) {
        let _scope = profile::Scope::new(profile::Phase::Insertion);
        let mut count = 0;
        let mut event = Vec::with_capacity(carried.len());
        let mut landing = Vec::with_capacity(carried.len());
        let mut group = BTreeMap::<usize, Vec<(usize, Carried)>>::new();
        for (position, value) in carried.into_iter().enumerate() {
            count += value.count;
            event.push(value.event);
            if value.list.is_empty() {
                landing.push(settle(value.landing, &[]));
            } else {
                landing.push(Vec::new());
                group
                    .entry(value.source)
                    .or_default()
                    .push((position, value));
            }
        }
        let taken = group
            .into_iter()
            .map(|(index, list)| {
                let set = std::mem::take(&mut self.trace[index]);
                let parent = std::mem::take(&mut self.parent[index]);
                (index, set, parent, list)
            })
            .collect::<Vec<_>>();
        let store = &self.capture;
        let inserted = map(executor, taken, |(index, mut set, mut parent, list)| {
            let start = set.len();
            let settled = list
                .into_iter()
                .map(|(position, carried)| {
                    let event = u32::try_from(carried.event).expect("fewer than 2^32 events");
                    let found = carried
                        .list
                        .into_iter()
                        .map(|(trace, from)| {
                            let (found, fresh) = set.insert_full(trace.share(store));
                            if fresh {
                                let from = u32::try_from(from).expect("fewer than 2^32 traces");
                                parent.push((event, from));
                            }
                            found
                        })
                        .collect::<Vec<_>>();
                    (position, settle(carried.landing, &found))
                })
                .collect::<Vec<_>>();
            (index, start, set, parent, settled)
        });
        let mut grown = Vec::new();
        for (index, start, set, parent, settled) in inserted {
            let end = set.len();
            self.trace[index] = set;
            self.parent[index] = parent;
            if start < end {
                grown.push((index, start..end));
            }
            for (position, value) in settled {
                landing[position] = value;
            }
        }
        for (event, landing) in event.into_iter().zip(landing) {
            self.crossed[event].extend(landing);
        }
        (count, grown)
    }
}
