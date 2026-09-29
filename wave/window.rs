use crate::dispatch::group;
use crate::engine::Engine;
use crate::failure::Failure;
use crate::hash;
use crate::setting::{BARE, FLAGGED, JOIN, MISSING, SUM, Setting, saturate};
use crate::store::Store;
use crate::table::Table;
use crate::upload::{Upload, refresh};
use crate::work::Work;
use metal::device::{Command, Memory};
use photonic::laser::net::{Net, Successor};
use photonic::runtime::Limit;

// A successor the host found for a marking whose kinds could join in one event: the marking's place
// in the window, the successor's place among the marking's, and then among the window's candidates,
// where its words start in the extra memory, after the two halves of its hash, how many events lead
// there and whether the limits admit it.
pub struct Joined {
    pub index: usize,
    pub order: u64,
    pub position: u64,
    pub extra: u32,
    pub weight: u64,
    pub admitted: bool,
}

// A run of markings whose successors are counted together: the first marking's id, how many, how
// many successors they have, the successors the host found, sorted by marking, and the markings
// with no successor at all.
pub struct Window {
    pub first: usize,
    pub size: usize,
    pub total: u64,
    pub joined: Vec<Joined>,
    pub end: Vec<usize>,
}

impl Window {
    // The end of the pass that starts at a marking: as many markings as fit within the candidates
    // a pass may hold, and at least one.
    pub fn bound(&self, start: &[u64], from: usize, most: u64) -> usize {
        let limit = start[from] + most;
        if self.total <= limit {
            return self.size;
        }
        let fitting = start[from + 1..self.size].partition_point(|&value| value <= limit);
        (from + fitting).max(from + 1)
    }

    // The candidates before a marking of the window, or all of them.
    pub fn before(&self, start: &[u64], index: usize) -> u64 {
        if index == self.size {
            return self.total;
        }
        start[index]
    }
}

fn admits(table: &Table, root: u32, kind: &[u32], limit: Limit) -> bool {
    let [world, occurrence, frame] = table.measure(kind).map(|value| value as usize);
    let base = table.base[root as usize] as usize;
    world <= limit.coherence && base + occurrence <= limit.occurrence && frame < limit.scope
}

impl Engine {
    // Makes room to count a window of at most size markings; when the running sums move to larger
    // memory, hands back the memory they leave, whose sums a pass that still reads them copies over.
    pub(crate) fn room(&self, work: &mut Work, size: usize) -> Result<Option<Memory>, Failure> {
        work.number.fit(&self.device, size)?;
        work.flagged.fit(&self.device, 2 * size)?;
        self.scan.prepare(&self.device, &mut work.level, size)?;
        work.start.swap(&self.device, size)
    }

    // Encodes the count of a window's successors, flagging markings the host must look at, and
    // their running sum.
    pub(crate) fn census<'device>(
        &self,
        command: &mut Command<'device>,
        store: &'device Store,
        tables: &'device Upload,
        work: &'device Work,
        setting: &Setting,
        size: usize,
    ) -> Result<(), Failure> {
        command.dispatch(
            &self.count,
            &[
                &store.arena.address,
                &store.offset.memory,
                &tables.key,
                &tables.lone,
                &tables.mask,
                &tables.need,
                &work.number.memory,
                &work.flagged.memory,
                &work.summary,
                &work.total,
            ],
            &setting.bytes(),
            [size, 1, 1],
            group(&self.count, size),
        );
        command.copy::<u64>(&work.number.memory, &work.start.memory, size)?;
        self.scan.encode(
            command,
            &work.start.memory,
            size,
            &work.total,
            SUM,
            &work.level,
        );
        Ok(())
    }

    // The values that count a window of markings on its own.
    pub(crate) fn alone(table: &Table, tables: &Upload, first: usize, size: usize) -> Setting {
        Setting {
            first: saturate(first),
            count: saturate(size),
            key: tables.slot,
            root: tables.root,
            rule: saturate(table.joining()),
            wide: u32::from(table.wide),
            next: saturate(first + size),
            room: u64::MAX,
            ..Setting::default()
        }
    }

    // Counts the successors of a window of markings and their running sum, unless the pass before
    // counted them already. The host grounds every flagged marking in order, as its own net would
    // on expanding it, and counts again if the tables lacked a part; then it finds the successors of
    // events joining several components, and sums again if there were any.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn count(
        &self,
        net: &mut Net,
        table: &mut Table,
        upload: &mut Option<Upload>,
        store: &mut Store,
        work: &mut Work,
        first: usize,
        size: usize,
        limit: Limit,
        counted: bool,
    ) -> Result<Window, Failure> {
        let mut counted = counted;
        let mut attempt = 0;
        let flagged = loop {
            if !counted {
                self.room(work, size)?;
                let tables = refresh(&self.device, table, upload)?;
                work.summary.edit::<u32>()[FLAGGED] = 0;
                let setting = Self::alone(table, tables, first, size);
                let mut command = self.device.command()?;
                command.reach(&store.arena.segment.iter().collect::<Vec<_>>());
                self.census(&mut command, store, tables, work, &setting, size)?;
                command.run()?;
            }
            counted = false;
            let count = work.summary.view::<u32>()[FLAGGED] as usize;
            let mut flagged = work.flagged.memory.view::<u32>()[..2 * count]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|&[index, state]| (index as usize, state))
                .collect::<Vec<_>>();
            flagged.sort_unstable();
            for &(index, state) in &flagged {
                if state & (MISSING | JOIN) != 0 {
                    let marking = store.marking(first + index);
                    table.prepare(net, &marking)?;
                }
            }
            if flagged.iter().all(|&(_, state)| state & MISSING == 0) {
                break flagged;
            }
            attempt += 1;
            if attempt > 1 {
                return Err(Failure::Missing);
            }
        };
        let mut joined = Vec::new();
        let mut end = Vec::new();
        let mut extra = Vec::<u32>::new();
        for (index, state) in flagged {
            let successor = if state & JOIN != 0 {
                let marking = store.marking(first + index);
                net.joined(&marking)?
            } else {
                Vec::new()
            };
            if state & BARE != 0 && successor.is_empty() {
                end.push(first + index);
            }
            if successor.is_empty() {
                continue;
            }
            let number = &mut work.number.memory.edit::<u64>()[index];
            let own = *number;
            *number += successor.len() as u64;
            for (order, Successor { marking, count }) in successor.into_iter().enumerate() {
                table.know(net, &marking);
                let digest = hash::digest(marking.root, &marking.kind);
                extra.extend([digest as u32, (digest >> 32) as u32]);
                joined.push(Joined {
                    index,
                    order: own + order as u64,
                    position: 0,
                    extra: saturate(extra.len()),
                    weight: count,
                    admitted: admits(table, marking.root, &marking.kind, limit),
                });
                extra.push(marking.root);
                extra.push(saturate(marking.kind.len()));
                extra.extend(&marking.kind);
            }
        }
        if !joined.is_empty() {
            let mut command = self.device.command()?;
            command.copy::<u64>(&work.number.memory, &work.start.memory, size)?;
            self.scan.encode(
                &mut command,
                &work.start.memory,
                size,
                &work.total,
                SUM,
                &work.level,
            );
            command.run()?;
            let start = work.start.memory.view::<u64>();
            for item in &mut joined {
                item.position = start[item.index] + item.order;
            }
        }
        work.extra.fit(&self.device, extra.len())?;
        work.extra.memory.edit::<u32>()[..extra.len()].copy_from_slice(&extra);
        Ok(Window {
            first,
            size,
            total: work.total.view::<u64>()[SUM as usize],
            joined,
            end,
        })
    }
}
