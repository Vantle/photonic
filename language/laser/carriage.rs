use super::capture;
use super::pool::{Demand, Reading};
use super::trace::{Trace, Wait};
use super::{CHUNK, Laser, Progress, map};
use crate::executor::Executor;
use crate::profile;
use hashing::Builder;
use indexmap::IndexSet;
use std::collections::{BTreeMap, VecDeque};
use std::num::NonZeroU32;
use std::ops::Range;

// A round can carry millions of traces. Carrying and inserting them a batch at a time keeps only
// one batch of new traces waiting, and every trace a batch inserts is known to the batches after
// it, so the traces, their positions and their landings are the ones a single batch would give.
// The same holds when a budget ends the round between batches and the next round carries the rest
// first.
const BATCH: usize = 1 << 20;

// The traces of an event's target, by position, that cross the event back to its source.
pub(super) struct Crossing {
    event: usize,
    range: Range<usize>,
}

// What carrying took: the traces carried, the new traces each source holds, in position order, and
// the crossings a budget left for the next round.
pub(super) struct Carriage {
    pub count: usize,
    pub grown: Vec<(usize, Range<usize>)>,
    pub rest: Vec<Crossing>,
}

// Where each trace of a crossing lands: nowhere when its frame did not exist before the event,
// at a trace the source already had, at one of the new traces the crossing carried, or, until the
// event's table learns an image it needs, nowhere yet.
#[derive(Clone, Copy)]
enum Landing {
    Lost,
    Known(usize),
    New(usize),
    Waiting,
}

// Each new trace remembers the position it was carried from, which is where its deduction goes on.
// The images most traces take across an event are their places' own, so a crossing is carried
// first, and only the images its waiting traces need are learned before they are carried.
struct Carried {
    event: usize,
    source: usize,
    first: usize,
    count: usize,
    landing: Vec<Landing>,
    list: Vec<(Trace, usize)>,
    demand: Demand,
}

// The crossings at the front of the queue that hold at most size traces, splitting the crossing
// that does not fit.
fn batch(queue: &mut VecDeque<Crossing>, size: usize) -> Vec<Crossing> {
    let mut batch = Vec::new();
    let mut room = size;
    while room > 0
        && let Some(crossing) = queue.pop_front()
    {
        if crossing.range.len() <= room {
            room -= crossing.range.len();
            batch.push(crossing);
            continue;
        }
        let split = crossing.range.start + room;
        batch.push(Crossing {
            event: crossing.event,
            range: crossing.range.start..split,
        });
        queue.push_front(Crossing {
            event: crossing.event,
            range: split..crossing.range.end,
        });
        room = 0;
    }
    batch
}

fn land(position: usize) -> NonZeroU32 {
    NonZeroU32::new(u32::try_from(position + 1).expect("fewer than 2^32 traces"))
        .expect("a position after zero")
}

// Inserts the traces a crossing carried into its source's traces, each new one remembering the
// crossing that brought it, and gives where every trace of the crossing landed.
fn absorb(
    store: &capture::Store,
    set: &mut IndexSet<Trace, Builder>,
    parent: &mut Vec<(u32, u32)>,
    carried: Carried,
) -> Vec<Option<NonZeroU32>> {
    let event = u32::try_from(carried.event).expect("fewer than 2^32 events");
    let found = carried
        .list
        .into_iter()
        .map(|(trace, from)| {
            let (found, fresh) = set.insert_full(trace.share(store));
            if fresh {
                parent.push((event, u32::try_from(from).expect("fewer than 2^32 traces")));
            }
            found
        })
        .collect::<Vec<_>>();
    settle(carried.landing, &found)
}

fn settle(landing: Vec<Landing>, position: &[usize]) -> Vec<Option<NonZeroU32>> {
    landing
        .into_iter()
        .map(|landing| match landing {
            Landing::Lost => None,
            Landing::Known(found) => Some(land(found)),
            Landing::New(index) => Some(land(position[index])),
            Landing::Waiting => unreachable!("a waiting trace lands once its images are learned"),
        })
        .collect()
}

impl Laser {
    // Where a trace of an event's target landed at its source, once the event carried it.
    pub(super) fn landing(&self, event: usize, position: usize) -> Option<usize> {
        let found = self.crossed[event].get(position).copied().flatten()?;
        Some(found.get() as usize - 1)
    }

    // Carries the crossings an earlier round left, then those of the changed configurations, a
    // batch at a time, and stops between batches once it has carried the allowance or the retained
    // records reach the record limit.
    pub(super) fn propagate(
        &mut self,
        executor: Option<&Executor>,
        left: Vec<Crossing>,
        changed: Vec<usize>,
        allowance: usize,
    ) -> Carriage {
        let mut queue = VecDeque::from(left);
        queue.extend(self.plan(changed));
        let mut count = 0;
        let mut grown = BTreeMap::<usize, Range<usize>>::new();
        while !queue.is_empty() && count < allowance && self.retained() < self.limit.record {
            let crossing = batch(&mut queue, BATCH.min(allowance - count));
            let carried = self.carry(executor, crossing);
            let carried = self.complete(executor, carried);
            let (value, range) = self.insert(executor, carried);
            self.capture.forget(executor);
            count += value;
            for (index, range) in range {
                self.traced += range.len();
                grown
                    .entry(index)
                    .and_modify(|known| known.end = range.end)
                    .or_insert(range);
            }
        }
        Carriage {
            count,
            grown: grown.into_iter().collect(),
            rest: queue.into(),
        }
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

    // Learns the images the waiting traces of a batch need, then carries those traces.
    fn complete(&mut self, executor: Option<&Executor>, mut carried: Vec<Carried>) -> Vec<Carried> {
        let mut demand = BTreeMap::<usize, Demand>::new();
        for value in &mut carried {
            if !value.demand.is_empty() {
                let found = std::mem::take(&mut value.demand);
                demand.entry(value.event).or_default().add(found);
            }
        }
        if demand.is_empty() {
            return carried;
        }
        self.prepare(executor, demand.into_iter().collect());
        let _scope = profile::Scope::new(profile::Phase::Carriage);
        map(executor, carried, |value| self.resume(value))
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
        let mut demand = Demand::default();
        let landing = crossing
            .range
            .clone()
            .map(|position| {
                let mut reading = Reading::new(&self.pool, passage, crossing.event, &mut demand);
                let carried = match set[position].carry(&mut reading, &self.capture) {
                    Ok(None) => return Landing::Lost,
                    Ok(Some(carried)) => carried,
                    Err(Wait) => return Landing::Waiting,
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
            first: crossing.range.start,
            count,
            landing,
            list,
            demand,
        }
    }

    // Carries a crossing's waiting traces once their images are learned, keeping its new traces in
    // the order of their positions.
    fn resume(&self, carried: Carried) -> Carried {
        if !carried
            .landing
            .iter()
            .any(|landing| matches!(landing, Landing::Waiting))
        {
            return carried;
        }
        let event = &self.event[carried.event];
        let set = &self.trace[event.target];
        let known = &self.trace[event.source];
        let passage = &self.passage[carried.event];
        let mut earlier = carried.list.into_iter().map(Some).collect::<Vec<_>>();
        let mut demand = Demand::default();
        let mut count = carried.count;
        let mut list = Vec::with_capacity(earlier.len());
        let landing = carried
            .landing
            .into_iter()
            .enumerate()
            .map(|(offset, landing)| match landing {
                Landing::Lost | Landing::Known(_) => landing,
                Landing::New(index) => {
                    list.push(earlier[index].take().expect("each new trace is kept once"));
                    Landing::New(list.len() - 1)
                }
                Landing::Waiting => {
                    let position = carried.first + offset;
                    let mut reading = Reading::new(&self.pool, passage, carried.event, &mut demand);
                    let Ok(Some(trace)) = set[position].carry(&mut reading, &self.capture) else {
                        unreachable!("a waiting trace exists at its source and has its images")
                    };
                    count += 1;
                    if let Some(found) = known.get_index_of(&trace) {
                        return Landing::Known(found);
                    }
                    list.push((trace, position));
                    Landing::New(list.len() - 1)
                }
            })
            .collect();
        Carried {
            count,
            landing,
            list,
            ..carried
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
                    (position, absorb(store, &mut set, &mut parent, carried))
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
