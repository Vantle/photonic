use crate::arena;
use crate::engine::Engine;
use crate::failure::Failure;
use crate::hash;
use crate::search::Search;
use crate::setting::{BARE, FLAGGED, JOIN, MISSING, OFFER, Setting, WINDOW, saturate};
use crate::work::Work;
use hashing::Builder;
use metal::device::{Command, Memory};
use photonic::laser::net::Successor;
use photonic::stop::Bound;
use std::collections::HashMap;

// A successor the host found for a marking whose kinds could join in one event: the marking's place
// in the window, the successor's place among the marking's, and then among the window's candidates,
// where its words start in the extra memory, after the two halves of its hash, how many events lead
// there and the limit that refuses it, if any.
pub struct Joined {
    pub index: usize,
    pub order: u64,
    pub position: u64,
    pub extra: u32,
    pub weight: u64,
    pub refused: Option<Bound>,
}

// A run of markings whose successors are counted together: the first marking's id, how many, how
// many successors they have, the successors the host found, sorted by marking, the markings with
// no successor at all, and whether the budget stopped the exploration at the marking after them.
pub struct Window {
    pub first: usize,
    pub size: usize,
    pub total: u64,
    pub joined: Vec<Joined>,
    pub end: Vec<usize>,
    pub spent: bool,
}

// A marking the host visited: its place in the window, what counting flagged it for, and the
// successors of its events joining several components.
struct Visit {
    index: usize,
    state: u32,
    successor: Vec<Successor>,
}

// The markings a pass covers: the window's markings from one to another, and the candidates between
// their running sums.
pub struct Range {
    pub from: usize,
    pub to: usize,
    pub begin: u64,
    pub end: u64,
}

impl Range {
    // How many candidates the pass decides.
    pub fn count(&self) -> usize {
        (self.end - self.begin) as usize
    }
}

impl Window {
    // The pass that starts at a marking: as many markings as fit within the candidates a pass may
    // hold, and at least one.
    pub fn range(&self, start: &[u64], from: usize, most: u64) -> Range {
        let limit = start[from] + most;
        let to = if self.total <= limit {
            self.size
        } else {
            let fitting = start[from + 1..self.size].partition_point(|&value| value <= limit);
            (from + fitting).max(from + 1)
        };
        Range {
            from,
            to,
            begin: start[from],
            end: if to == self.size {
                self.total
            } else {
                start[to]
            },
        }
    }

    // The successors the host found for the markings of a range.
    pub fn found(&self, range: &Range) -> &[Joined] {
        let low = self.joined.partition_point(|item| item.index < range.from);
        let high = self.joined.partition_point(|item| item.index < range.to);
        &self.joined[low..high]
    }
}

// The memory a window's running sums and join routes leave when counting a larger window moves
// them, which the pass that still reads them copies over before it does.
pub struct Left {
    pub start: Option<Memory>,
    pub route: Option<Memory>,
}

impl Engine {
    // Makes room to count a window of at most size markings, and hands back the memory the running
    // sums and the join routes leave when they move to larger memory.
    pub(crate) fn room(&self, work: &mut Work, size: usize) -> Result<Left, Failure> {
        work.number.fit(&self.device, size)?;
        work.flagged.fit(&self.device, 2 * size)?;
        self.scan.prepare(&self.device, &mut work.level, size)?;
        Ok(Left {
            start: work.start.swap(&self.device, size)?,
            route: work.route.swap(&self.device, size)?,
        })
    }

    // Encodes the running sum of a window's successor counts.
    fn sum<'device>(
        &self,
        command: &mut Command<'device>,
        work: &'device Work,
        size: usize,
    ) -> Result<(), Failure> {
        command.copy::<u64>(&work.number.memory, &work.start.memory, size)?;
        self.scan.encode(
            command,
            &work.start.memory,
            size,
            &work.total,
            WINDOW,
            &work.level,
        )
    }

    // Encodes the count of a window's successors, flagging markings the host must look at, and
    // their running sum.
    pub(crate) fn census<'device>(
        &self,
        command: &mut Command<'device>,
        search: &'device Search<'_>,
        setting: &Setting,
    ) -> Result<(), Failure> {
        let size = setting.count as usize;
        command.dispatch(
            &self.count,
            &[
                &search.store.arena.address,
                &search.store.offset.memory,
                search.upload.key.memory(),
                &search.upload.lone.memory,
                search.upload.catalog.memory(),
                &search.upload.part.memory,
                &search.upload.member.memory,
                &search.upload.coherence.memory,
                &search.upload.span.memory,
                &search.upload.reach.memory,
                &search.upload.rule.memory,
                &search.upload.arity.memory,
                &search.work.number.memory,
                &search.work.flagged.memory,
                &search.work.summary,
                &search.work.total,
                &search.work.route.memory,
            ],
            &setting.byte(),
            [size, 1, 1],
            self.count.group([size, 1, 1]),
        )?;
        self.sum(command, &search.work, size)
    }

    // The values that find the tables: the slots of the component index and of the part catalog, the
    // roots, the joining rules, and the offers and parts a marking's joins may take on the GPU.
    pub(crate) fn table(&self, search: &Search<'_>) -> Setting {
        Setting {
            key: search.upload.key.slot,
            catalog: search.upload.catalog.slot,
            root: search.upload.root,
            rule: saturate(search.table.arity.len()),
            join: saturate(self.tuning.join.min(OFFER)),
            ..Setting::default()
        }
    }

    // The values that count a window of markings on its own.
    pub(crate) fn alone(&self, search: &Search<'_>, first: usize, size: usize) -> Setting {
        Setting {
            first: saturate(first),
            count: saturate(size),
            next: saturate(first + size),
            room: u64::MAX,
            ..self.table(search)
        }
    }

    // Counts the successors of a window of markings and their running sum, unless the pass before
    // counted them already, has the host visit every marking counting flagged, and writes the
    // successors of events joining several components. A marking whose parts the budget cannot
    // ground ends the window before it, as it ends the host's exploration.
    pub(crate) fn window(
        &self,
        search: &mut Search<'_>,
        first: usize,
        size: usize,
        counted: bool,
    ) -> Result<Window, Failure> {
        let (visited, kept) = self.visit(search, first, size, counted)?;
        let end = visited
            .iter()
            .filter(|visit| visit.state & BARE != 0 && visit.successor.is_empty())
            .map(|visit| first + visit.index)
            .collect();
        let joined = self.join(search, kept, visited)?;
        Ok(Window {
            first,
            size: kept,
            total: search.work.total.view::<u64>()[WINDOW],
            joined,
            end,
            spent: kept < size,
        })
    }

    // Visits every marking of a window that counting flagged, in order, as the host's own net would
    // on expanding it, and gives the visits with the size of the window they cover. When the tables
    // lacked a part, the visits ground it and the window is counted again, once, keeping what the
    // visits found, so no host join is taken twice; a marking whose parts or host join the budget
    // cannot take ends the window before it, and the shorter window is counted again.
    fn visit(
        &self,
        search: &mut Search<'_>,
        first: usize,
        mut size: usize,
        mut counted: bool,
    ) -> Result<(Vec<Visit>, usize), Failure> {
        let mut found = HashMap::<usize, Vec<Successor>, Builder>::default();
        for _ in 0..2 {
            if !counted {
                self.room(&mut search.work, size)?;
                search.upload.refresh(&self.device, &mut search.table)?;
                search.work.summary.edit::<u32>()[FLAGGED] = 0;
                let setting = self.alone(search, first, size);
                let mut command = self.command(&search.store.arena)?;
                self.census(&mut command, search, &setting)?;
                command.run()?;
            }
            counted = false;
            let count = search.work.summary.view::<u32>()[FLAGGED] as usize;
            let mut flagged = search.work.flagged.memory.view::<u32>()[..2 * count]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|&[index, state]| (index as usize, state))
                .collect::<Vec<_>>();
            flagged.sort_unstable();
            let mut visited = Vec::with_capacity(flagged.len());
            let mut shortened = false;
            for (index, state) in flagged {
                if state & (MISSING | JOIN) == 0 {
                    visited.push(Visit {
                        index,
                        state,
                        successor: Vec::new(),
                    });
                    continue;
                }
                let successor = match found.remove(&index) {
                    Some(successor) => successor,
                    None => {
                        let marking = search.store.marking(first + index);
                        let Some(expansion) =
                            search
                                .table
                                .prepare(search.net, &marking, search.allowance)?
                        else {
                            size = index;
                            shortened = true;
                            break;
                        };
                        expansion.successor
                    }
                };
                visited.push(Visit {
                    index,
                    state,
                    successor,
                });
            }
            if !shortened && visited.iter().all(|visit| visit.state & MISSING == 0) {
                return Ok((visited, size));
            }
            found = visited
                .into_iter()
                .filter(|visit| visit.state & (MISSING | JOIN) != 0)
                .map(|visit| (visit.index, visit.successor))
                .collect();
        }
        Err(Failure::Missing)
    }

    // Writes the words of the successors the host found to the extra memory and places each after
    // the successors the tables give its marking, summing the window again if there were any.
    fn join(
        &self,
        search: &mut Search<'_>,
        size: usize,
        visited: Vec<Visit>,
    ) -> Result<Vec<Joined>, Failure> {
        let mut joined = Vec::new();
        let mut extra = Vec::<u32>::new();
        for visit in visited {
            if visit.successor.is_empty() {
                continue;
            }
            let number = &mut search.work.number.memory.edit::<u64>()[visit.index];
            let own = *number;
            *number += visit.successor.len() as u64;
            for (order, Successor { marking, count }) in visit.successor.into_iter().enumerate() {
                search.table.know(search.net, &marking);
                let digest = hash::digest(marking.root, &marking.kind);
                extra.extend([digest as u32, (digest >> 32) as u32]);
                joined.push(Joined {
                    index: visit.index,
                    order: own + order as u64,
                    position: 0,
                    extra: saturate(extra.len()),
                    weight: count,
                    refused: search.net.refuse(&marking, search.limit),
                });
                extra.extend(arena::word(&marking));
            }
        }
        if !joined.is_empty() {
            let mut command = self.device.command()?;
            self.sum(&mut command, &search.work, size)?;
            command.run()?;
            let start = search.work.start.memory.view::<u64>();
            for item in &mut joined {
                item.position = start[item.index] + item.order;
            }
        }
        search.work.extra.fit(&self.device, extra.len())?;
        search.work.extra.memory.edit::<u32>()[..extra.len()].copy_from_slice(&extra);
        Ok(joined)
    }
}
