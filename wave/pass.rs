use crate::engine::{Engine, Tally, group};
use crate::failure::Failure;
use crate::scan::whole;
use crate::setting::{
    BLOCKED, CANDIDATE, COPIES, FLAGGED, JOINED, LIMITED, REFUSED, SHIFT, Setting, TAG, WIDTH,
    WORDS, saturate,
};
use crate::store::Store;
use crate::table::Table;
use crate::upload::{Upload, refresh};
use crate::window::{Joined, Window};
use crate::work::Work;
use metal::device::Command;
use photonic::runtime::Limit;

// The markings a pass covers: the window's markings from one to another, and the candidates between
// their running sums.
pub struct Range {
    pub from: usize,
    pub to: usize,
    pub begin: u64,
    pub end: u64,
}

// Writes the successors the host found for the pass's markings where the running sum places them.
fn host(work: &mut Work, joined: &[Joined], first: usize, begin: u64) {
    let record = work.record.memory.edit::<u32>();
    for item in joined {
        let c = (item.position - begin) as usize;
        let flag = if item.admitted {
            JOINED
        } else {
            JOINED | LIMITED
        };
        record[3 * c..3 * c + 3].copy_from_slice(&[saturate(first + item.index), item.extra, flag]);
    }
}

// The events a pass's admitted candidates stand for, counted on the host when the configuration
// limit may have refused some of them after the kernels summed them.
fn recount(work: &mut Work, table: &Table, joined: &[Joined], begin: u64, count: usize) -> u64 {
    let target = work.target.memory.view::<u32>()[..count].to_vec();
    let record = work.record.memory.view::<u32>();
    let mut event = (0..count)
        .filter(|&index| target[index] != BLOCKED && record[3 * index + 2] & JOINED == 0)
        .map(|index| {
            let copies = u64::from(record[3 * index + 2] & COPIES);
            copies * u64::from(table.entry[record[3 * index + 1] as usize + 1])
        })
        .sum::<u64>();
    for item in joined {
        if target[(item.position - begin) as usize] != BLOCKED {
            event += item.weight;
        }
    }
    event
}

impl Engine {
    // Decides one pass of candidates in one command: records them, finds or claims their markings,
    // ranks the new ones a threadgroup at a time, then writes them and points every candidate at its
    // marking; only new markings that overflow the arena's last segment take a second command. The last pass of a window also counts the
    // next window, which starts at ahead, in the same command, and tells whether it did.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn pass(
        &self,
        table: &mut Table,
        upload: &mut Option<Upload>,
        store: &mut Store,
        work: &mut Work,
        window: &Window,
        range: Range,
        ahead: Option<usize>,
        limit: Limit,
        cycle: bool,
        tally: &mut Tally,
    ) -> Result<bool, Failure> {
        let count = (range.end - range.begin) as usize;
        if count == 0 {
            return Ok(false);
        }
        if count >= CANDIDATE {
            return Err(Failure::Candidate { count });
        }
        let allowed = limit.configuration.saturating_sub(store.count);
        let most = count.min(allowed);
        if store.count + most >= TAG as usize - 1 {
            return Err(Failure::Configuration {
                count: store.count + most,
            });
        }
        let width = whole(&self.expand);
        let groups = (range.to - range.from).div_ceil(width);
        let span = whole(&self.tally).min(whole(&self.place));
        let blocks = count.div_ceil(4 * span);
        work.record.fit(&self.device, 3 * count)?;
        work.sums.fit(&self.device, range.to - range.from)?;
        work.target.fit(&self.device, count)?;
        work.slot.fit(&self.device, count)?;
        work.event.fit(&self.device, groups)?;
        work.block.fit(&self.device, blocks)?;
        self.scan.prepare(&self.device, &mut work.level, blocks)?;
        let offset = store.offset.swap(&self.device, store.count + most)?;
        let next = ahead
            .map(|first| (first, (store.count + most).saturating_sub(first)))
            .map(|(first, size)| (first, size.min(self.shape.window)))
            .filter(|&(_, size)| size > 0);
        let start = match next {
            Some((_, size)) => self.room(work, size)?,
            None => None,
        };
        let low = window
            .joined
            .partition_point(|item| item.index < range.from);
        let high = window.joined.partition_point(|item| item.index < range.to);
        let joined = &window.joined[low..high];
        host(work, joined, window.first, range.begin);
        tally.refused |= joined.iter().any(|item| !item.admitted);
        let (safe, likely) = if allowed > 0 {
            let fresh = (count as f64 * store.fresh).ceil() as usize;
            (store.count + count + 1, store.count + fresh.min(count))
        } else {
            (store.count + 1, store.count)
        };
        let grown = self.grow(store, safe, likely)?;
        let tables = refresh(&self.device, table, upload)?;
        work.summary.edit::<u32>()[REFUSED] = 0;
        work.summary.edit::<u32>()[FLAGGED] = 0;
        let room = store.arena.room();
        let setting = Setting {
            base: range.begin,
            arena: store.arena.base(),
            split: u64::MAX,
            room: room as u64,
            first: saturate(window.first + range.from),
            count: saturate(range.to - range.from),
            shift: saturate(range.from),
            key: tables.slot,
            root: tables.root,
            rule: saturate(table.joining()),
            wide: u32::from(table.wide),
            coherence: saturate(limit.coherence),
            occurrence: saturate(limit.occurrence),
            scope: saturate(limit.scope),
            bucket: saturate(store.slot / WIDTH),
            next: saturate(store.count),
            allowed: saturate(allowed),
            ..Setting::default()
        };
        let decide = Setting {
            count: saturate(count),
            ..setting
        };
        let mut command = self.device.command()?;
        command.reach(&store.arena.segment.iter().collect::<Vec<_>>());
        if let Some(old) = &offset {
            command.copy::<u64>(old, &store.offset.memory, store.count)?;
        }
        if let Some(old) = &start {
            command.copy::<u64>(old, &work.start.memory, window.size)?;
        }
        if grown {
            self.fill(&mut command, store)?;
        }
        command.dispatch(
            &self.expand,
            &[
                &store.arena.address,
                &store.offset.memory,
                &tables.entry,
                &tables.key,
                &tables.lone,
                &tables.size,
                &tables.base,
                &work.start.memory,
                &work.record.memory,
                &work.sums.memory,
                &work.event.memory,
                &work.summary,
            ],
            &setting.bytes(),
            [groups * width, 1, 1],
            [width, 1, 1],
        );
        command.dispatch(
            &self.insert,
            &[
                &store.arena.address,
                &store.offset.memory,
                &tables.entry,
                &work.extra.memory,
                &work.record.memory,
                &work.sums.memory,
                &work.target.memory,
                &store.table,
                &work.slot.memory,
                &work.summary,
                &store.table,
            ],
            &decide.bytes(),
            [count, 1, 1],
            group(&self.insert, count),
        );
        command.dispatch(
            &self.tally,
            &[
                &store.arena.address,
                &store.offset.memory,
                &tables.entry,
                &work.extra.memory,
                &work.record.memory,
                &store.table,
                &work.slot.memory,
                &work.block.memory,
            ],
            &decide.bytes(),
            [blocks * span, 1, 1],
            [span, 1, 1],
        );
        self.scan.encode(
            &mut command,
            &work.block.memory,
            blocks,
            &work.total,
            0,
            &work.level,
        );
        self.settle(&mut command, store, tables, work, &decide, span);
        let later = |room: u64| {
            next.map(|(first, size)| Setting {
                first: saturate(first),
                count: saturate(size),
                next: saturate(store.count),
                allowed: saturate(allowed),
                room,
                ..Self::alone(table, tables, first, size)
            })
        };
        if let Some(setting) = later(room as u64) {
            self.census(
                &mut command,
                store,
                tables,
                work,
                &setting,
                setting.count as usize,
            )?;
        }
        command.run()?;
        let total = work.total.view::<u64>()[0];
        let (winner, words) = ((total >> SHIFT) as usize, (total & WORDS) as usize);
        if words > room {
            let kept = {
                let block = &work.block.memory.view::<u64>()[..blocks];
                let last = block[1..].partition_point(|&value| value & WORDS <= room as u64);
                block[last] & WORDS
            };
            let place = store.arena.split(&self.device, kept, words)?;
            let decide = Setting {
                arena: place.base,
                split: place.split,
                overflow: place.overflow,
                room: u64::MAX,
                ..decide
            };
            let mut command = self.device.command()?;
            command.reach(&store.arena.segment.iter().collect::<Vec<_>>());
            self.settle(&mut command, store, tables, work, &decide, span);
            if let Some(setting) = later(u64::MAX) {
                self.census(
                    &mut command,
                    store,
                    tables,
                    work,
                    &setting,
                    setting.count as usize,
                )?;
            }
            command.run()?;
        } else {
            store.arena.advance(words);
        }
        tally.event += if allowed >= count {
            let summed = work.event.memory.view::<u64>()[..groups]
                .iter()
                .sum::<u64>();
            let host = joined
                .iter()
                .filter(|item| item.admitted)
                .map(|item| item.weight)
                .sum::<u64>();
            summed + host
        } else {
            recount(work, table, joined, range.begin, count)
        };
        tally.refused |= work.summary.view::<u32>()[REFUSED] != 0;
        store.count += winner.min(allowed);
        store.fresh = winner as f64 / count as f64;
        if cycle {
            let target = work.target.memory.view::<u32>()[..count].to_vec();
            let record = work.record.memory.view::<u32>();
            for (index, &aim) in target.iter().enumerate() {
                tally.reach(record[3 * index] as usize);
                if aim != BLOCKED {
                    tally.edge.push(aim);
                }
            }
        }
        Ok(next.is_some())
    }

    // Encodes the kernels that number and write the pass's new markings, a threadgroup of span
    // threads and four candidates a thread at a time, and then point every candidate at its
    // marking.
    fn settle<'device>(
        &self,
        command: &mut Command<'device>,
        store: &'device Store,
        tables: &'device Upload,
        work: &'device Work,
        setting: &Setting,
        span: usize,
    ) {
        let count = setting.count as usize;
        command.dispatch(
            &self.place,
            &[
                &store.arena.address,
                &store.offset.memory,
                &tables.entry,
                &work.extra.memory,
                &work.record.memory,
                &work.target.memory,
                &store.table,
                &work.slot.memory,
                &work.block.memory,
                &work.total,
            ],
            &setting.bytes(),
            [count.div_ceil(4 * span) * span, 1, 1],
            [span, 1, 1],
        );
        command.dispatch(
            &self.point,
            &[
                &work.target.memory,
                &store.table,
                &work.slot.memory,
                &work.summary,
                &work.total,
            ],
            &setting.bytes(),
            [count, 1, 1],
            group(&self.point, count),
        );
    }
}
