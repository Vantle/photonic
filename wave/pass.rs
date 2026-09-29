use crate::dispatch::whole;
use crate::engine::Engine;
use crate::failure::Failure;
use crate::search::Search;
use crate::setting::{
    BLOCKED, BOUND, CANDIDATE, FLAGGED, JOINED, LIMITED, REFUSED, SHIFT, Setting, TAG, WIDTH,
    WINNER, WORD, saturate,
};
use crate::window::{Joined, Range, Window};
use crate::work::Work;
use metal::device::Command;
use photonic::laser::net::Cycle;

// Writes the successors the host found for the pass's markings where the running sum places them.
fn host(work: &mut Work, joined: &[Joined], first: usize, begin: u64) {
    let record = work.record.memory.edit::<u32>();
    for item in joined {
        let candidate = (item.position - begin) as usize;
        let flag = if item.admitted {
            JOINED
        } else {
            JOINED | LIMITED
        };
        record[3 * candidate..3 * candidate + 3].copy_from_slice(&[
            saturate(first + item.index),
            item.extra,
            flag,
        ]);
    }
}

impl Engine {
    // Decides one pass of candidates in one command: records them, finds or claims their markings,
    // ranks the new ones a threadgroup at a time, then writes them and points every candidate at its
    // marking; only new markings that overflow the arena's last segment take a second command. The
    // last pass of a window also counts the next window, which starts at ahead, in the same
    // command, and tells whether it did.
    pub(crate) fn pass(
        &self,
        search: &mut Search<'_>,
        window: &Window,
        range: Range,
        ahead: Option<usize>,
    ) -> Result<bool, Failure> {
        if search.cycle == Cycle::Find {
            let origin = search.tally.edge.len() as u64;
            let start = search.work.start.memory.view::<u64>();
            search.tally.first.extend(
                start[range.from..range.to]
                    .iter()
                    .map(|&value| origin + (value - range.begin)),
            );
        }
        let count = (range.end - range.begin) as usize;
        if count == 0 {
            return Ok(false);
        }
        if count >= CANDIDATE {
            return Err(Failure::Candidate { count });
        }
        let allowed = search
            .limit
            .configuration
            .saturating_sub(search.store.count);
        let most = count.min(allowed);
        if search.store.count + most >= TAG as usize - 1 {
            return Err(Failure::Configuration {
                count: search.store.count + most,
            });
        }
        let width = whole(&self.expand);
        let group = (range.to - range.from).div_ceil(width);
        let span = self.span();
        let block = count.div_ceil(4 * span);
        let work = &mut search.work;
        work.record.fit(&self.device, 3 * count)?;
        work.sum.fit(&self.device, range.to - range.from)?;
        work.target.fit(&self.device, count)?;
        work.slot.fit(&self.device, count)?;
        work.share.fit(&self.device, count)?;
        work.event.fit(&self.device, group.max(block))?;
        work.block.fit(&self.device, block)?;
        self.scan.prepare(&self.device, &mut work.level, block)?;
        let offset = search
            .store
            .offset
            .swap(&self.device, search.store.count + most)?;
        let next = ahead
            .map(|first| (first, (search.store.count + most).saturating_sub(first)))
            .map(|(first, size)| (first, size.min(self.tuning.window)))
            .filter(|&(_, size)| size > 0);
        let start = match next {
            Some((_, size)) => self.room(&mut search.work, size)?,
            None => None,
        };
        let joined = window.found(&range);
        host(&mut search.work, joined, window.first, range.begin);
        if joined.iter().any(|item| !item.admitted) {
            search.work.summary.edit::<u32>()[REFUSED] = 1;
        }
        let (safe, likely) = if allowed > 0 {
            let fresh = (count as f64 * search.store.fresh).ceil() as usize;
            (
                search.store.count + count + 1,
                search.store.count + fresh.min(count),
            )
        } else {
            (search.store.count + 1, search.store.count)
        };
        let grown = search.store.grow(&self.device, safe, likely)?;
        search.upload.refresh(&self.device, &mut search.table)?;
        search.work.summary.edit::<u32>()[FLAGGED] = 0;
        search.work.total.edit::<u64>()[BOUND] = u64::MAX;
        let room = search.store.arena.room();
        let setting = Setting {
            base: range.begin,
            arena: search.store.arena.base(),
            split: u64::MAX,
            room: room as u64,
            first: saturate(window.first + range.from),
            count: saturate(range.to - range.from),
            shift: saturate(range.from),
            key: search.upload.slot,
            root: search.upload.root,
            rule: saturate(search.table.joining()),
            wide: u32::from(search.table.wide),
            coherence: saturate(search.limit.coherence),
            occurrence: saturate(search.limit.occurrence),
            scope: saturate(search.limit.scope),
            bucket: saturate(search.store.slot / WIDTH),
            next: saturate(search.store.count),
            allowed: saturate(allowed),
            ..Setting::default()
        };
        let decide = Setting {
            count: saturate(count),
            ..setting
        };
        let later = next.map(|(first, size)| Setting {
            next: saturate(search.store.count),
            allowed: saturate(allowed),
            ..Self::alone(search, first, size)
        });
        let mut command = self.device.command()?;
        command.reach(&search.store.arena.segment.iter().collect::<Vec<_>>());
        if let Some(old) = &offset {
            command.copy::<u64>(old, &search.store.offset.memory, search.store.count)?;
        }
        if let Some(old) = &start {
            command.copy::<u64>(old, &search.work.start.memory, window.size)?;
        }
        if grown {
            self.fill(&mut command, search)?;
        }
        let (store, upload, work) = (&search.store, &search.upload, &search.work);
        command.dispatch(
            &self.expand,
            &[
                &store.arena.address,
                &store.offset.memory,
                &upload.entry.memory,
                &upload.key.memory,
                &upload.lone.memory,
                &upload.size.memory,
                &upload.base.memory,
                &work.start.memory,
                &work.record.memory,
                &work.sum.memory,
                &work.event.memory,
                &work.summary,
            ],
            &setting.byte(),
            [group * width, 1, 1],
            [width, 1, 1],
        );
        command.dispatch(
            &self.insert,
            &[
                &store.arena.address,
                &store.offset.memory,
                &upload.entry.memory,
                &work.extra.memory,
                &work.record.memory,
                &work.sum.memory,
                &work.target.memory,
                &store.table,
                &work.slot.memory,
                &work.summary,
                &store.table,
            ],
            &decide.byte(),
            [count, 1, 1],
            self.insert.group([count, 1, 1]),
        );
        command.dispatch(
            &self.tally,
            &[
                &store.arena.address,
                &store.offset.memory,
                &upload.entry.memory,
                &work.extra.memory,
                &work.record.memory,
                &store.table,
                &work.slot.memory,
                &work.share.memory,
                &work.block.memory,
            ],
            &decide.byte(),
            [block * span, 1, 1],
            [span, 1, 1],
        );
        self.scan.encode(
            &mut command,
            &work.block.memory,
            block,
            &work.total,
            WINNER,
            &work.level,
        );
        if allowed < count {
            let bound = Setting {
                count: saturate(block),
                allowed: saturate(allowed),
                ..Setting::default()
            };
            command.dispatch(
                &self.bound,
                &[&work.block.memory, &work.total],
                &bound.byte(),
                [block + 1, 1, 1],
                self.bound.group([block + 1, 1, 1]),
            );
        }
        self.settle(
            &mut command,
            search,
            &decide,
            later.map(|setting| Setting {
                room: room as u64,
                ..setting
            }),
        )?;
        command.run()?;
        let total = search.work.total.view::<u64>();
        let winner = (total[WINNER] >> SHIFT) as usize;
        let word = total[BOUND].min(total[WINNER] & WORD) as usize;
        if word > room {
            let kept = {
                let prefix = &search.work.block.memory.view::<u64>()[..block];
                let last = prefix[1..].partition_point(|&value| value & WORD <= room as u64);
                prefix[last] & WORD
            };
            let overflow = search.store.arena.split(&self.device, kept, word)?;
            let decide = Setting {
                split: kept,
                overflow,
                room: u64::MAX,
                ..decide
            };
            let mut command = self.device.command()?;
            command.reach(&search.store.arena.segment.iter().collect::<Vec<_>>());
            self.settle(&mut command, search, &decide, later)?;
            command.run()?;
        } else {
            search.store.arena.advance(word);
        }
        let weighed = if allowed < count { block } else { group };
        let summed = search.work.event.memory.view::<u64>()[..weighed]
            .iter()
            .sum::<u64>();
        let target = search.work.target.memory.view::<u32>();
        let found = joined
            .iter()
            .filter(|item| target[(item.position - range.begin) as usize] != BLOCKED)
            .map(|item| item.weight)
            .sum::<u64>();
        search.tally.event += summed + found;
        if search.cycle == Cycle::Find {
            search.tally.edge.extend_from_slice(&target[..count]);
        }
        search.store.count += winner.min(allowed);
        search.store.fresh = winner as f64 / count as f64;
        Ok(next.is_some())
    }

    // The threads of a group for the kernels that take four candidates a thread; tallying and
    // placing must share their groups, since placing ranks each winner within the group tallying
    // summed.
    fn span(&self) -> usize {
        whole(&self.tally)
            .min(whole(&self.place))
            .min(whole(&self.weigh))
    }

    // Encodes the kernels that number and write the pass's new markings, a threadgroup and four
    // candidates a thread at a time, and point every candidate at its marking; when the
    // configuration limit may refuse candidates, their events are summed once they point at their
    // markings. The next window, when there is one, is counted after them.
    fn settle<'device>(
        &self,
        command: &mut Command<'device>,
        search: &'device Search<'_>,
        decide: &Setting,
        later: Option<Setting>,
    ) -> Result<(), Failure> {
        let (store, upload, work) = (&search.store, &search.upload, &search.work);
        let count = decide.count as usize;
        let span = self.span();
        let block = count.div_ceil(4 * span);
        command.dispatch(
            &self.place,
            &[
                &store.arena.address,
                &store.offset.memory,
                &upload.entry.memory,
                &work.extra.memory,
                &work.record.memory,
                &work.target.memory,
                &store.table,
                &work.slot.memory,
                &work.share.memory,
                &work.block.memory,
                &work.total,
            ],
            &decide.byte(),
            [block * span, 1, 1],
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
            &decide.byte(),
            [count, 1, 1],
            self.point.group([count, 1, 1]),
        );
        if decide.allowed < decide.count {
            command.dispatch(
                &self.weigh,
                &[
                    &upload.entry.memory,
                    &work.record.memory,
                    &work.target.memory,
                    &work.event.memory,
                    &work.total,
                ],
                &decide.byte(),
                [block * span, 1, 1],
                [span, 1, 1],
            );
        }
        if let Some(setting) = later {
            self.census(command, search, &setting)?;
        }
        Ok(())
    }
}
