use crate::engine::Engine;
use crate::failure::Failure;
use crate::search::Search;
use crate::setting::{
    BLOCKED, BOUND, CANDIDATE, FLAGGED, JOINED, LIMITED, SHIFT, Setting, TAG, WIDTH, WINNER, WORD,
    saturate,
};
use crate::window::{Joined, Range, Window};
use crate::work::Work;
use metal::device::{Command, Memory};
use photonic::laser::net::Cycle;
use photonic::stop::Bound;

// Writes the successors the host found for the pass's markings where the running sum places them,
// marked when the limits refuse them, as expanding records the successors the tables give.
fn host(work: &mut Work, joined: &[Joined], first: usize, begin: u64) {
    let record = work.record.memory.edit::<u32>();
    for item in joined {
        let candidate = (item.position - begin) as usize;
        let flag = match item.refused {
            Some(_) => JOINED | LIMITED,
            None => JOINED,
        };
        record[3 * candidate..3 * candidate + 3].copy_from_slice(&[
            saturate(first + item.index),
            item.extra,
            flag,
        ]);
    }
}

// How many new markings the configuration limit admits in a pass of count candidates, and how many
// markings there are at most once the pass numbers its winners; a pass numbers its candidates
// below CANDIDATE and the table its markings below TAG.
fn admit(search: &Search<'_>, count: usize) -> Result<(usize, usize), Failure> {
    if count >= CANDIDATE {
        return Err(Failure::Candidate { count });
    }
    let allowed = search
        .limit
        .configuration
        .saturating_sub(search.store.count);
    let most = search.store.count + count.min(allowed);
    if most >= TAG as usize - 1 {
        return Err(Failure::Configuration { count: most });
    }
    Ok((allowed, most))
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
            search
                .tally
                .source(search.work.start.memory.view::<u64>(), &range);
        }
        let count = range.count();
        if count == 0 {
            return Ok(false);
        }
        let (allowed, most) = admit(search, count)?;
        let next = ahead
            .map(|first| (first, most.saturating_sub(first).min(self.tuning.window)))
            .filter(|&(_, size)| size > 0);
        let (offset, start) = self.reserve(search, &range, most, next)?;
        let joined = window.found(&range);
        host(&mut search.work, joined, window.first, range.begin);
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
        let setting = Setting {
            base: range.begin,
            arena: search.store.arena.base(),
            split: u64::MAX,
            room: search.store.arena.room() as u64,
            first: saturate(window.first + range.from),
            count: saturate(range.to - range.from),
            shift: saturate(range.from),
            coherence: saturate(search.limit.coherence),
            occurrence: saturate(search.limit.occurrence),
            scope: saturate(search.limit.scope),
            bucket: saturate(search.store.slot / WIDTH),
            next: saturate(search.store.count),
            allowed: saturate(allowed),
            ..self.table(search)
        };
        let decide = Setting {
            count: saturate(count),
            ..setting
        };
        let later = next.map(|(first, size)| Setting {
            next: saturate(search.store.count),
            allowed: saturate(allowed),
            ..self.alone(search, first, size)
        });
        let mut command = self.command(&search.store.arena)?;
        if let Some(old) = &offset {
            command.copy::<u64>(old, &search.store.offset.memory, search.store.count)?;
        }
        if let Some(old) = &start {
            command.copy::<u64>(old, &search.work.start.memory, window.size)?;
        }
        if grown {
            self.fill(&mut command, &search.store)?;
        }
        self.claim(&mut command, search, &setting, &decide)?;
        self.settle(
            &mut command,
            search,
            &decide,
            later.map(|setting| Setting {
                room: decide.room,
                ..setting
            }),
        )?;
        command.run()?;
        self.spill(search, &decide, later)?;
        self.account(search, joined, &range, allowed);
        Ok(next.is_some())
    }

    // The threadgroups that expand markings, one a thread.
    fn group(&self, marking: usize) -> usize {
        marking.div_ceil(self.width)
    }

    // The threadgroups that take candidates, four a thread.
    fn block(&self, count: usize) -> usize {
        count.div_ceil(4 * self.span)
    }

    // Makes room for a pass's candidates, for the offsets of the most markings there can be once it
    // numbers its winners and for counting the next window, when there is one; hands back the
    // memory the offsets and the running sums leave, whose values the pass copies over.
    fn reserve(
        &self,
        search: &mut Search<'_>,
        range: &Range,
        most: usize,
        next: Option<(usize, usize)>,
    ) -> Result<(Option<Memory>, Option<Memory>), Failure> {
        let count = range.count();
        let marking = range.to - range.from;
        let group = self.group(marking);
        let block = self.block(count);
        let work = &mut search.work;
        work.record.fit(&self.device, 3 * count)?;
        work.sum.fit(&self.device, marking)?;
        work.target.fit(&self.device, count)?;
        work.slot.fit(&self.device, count)?;
        work.share.fit(&self.device, count)?;
        work.event.fit(&self.device, group.max(block))?;
        work.block.fit(&self.device, block)?;
        self.scan.prepare(&self.device, &mut work.level, block)?;
        let offset = search.store.offset.swap(&self.device, most)?;
        let start = match next {
            Some((_, size)) => self.room(&mut search.work, size)?,
            None => None,
        };
        Ok((offset, start))
    }

    // Encodes the kernels that record a pass's candidates from the tables, find the marking each
    // one equals or claim a slot for it, and sum each threadgroup's winners and their words, with
    // the words of the threadgroups the configuration limit admits when it may refuse winners.
    fn claim<'device>(
        &self,
        command: &mut Command<'device>,
        search: &'device Search<'_>,
        setting: &Setting,
        decide: &Setting,
    ) -> Result<(), Failure> {
        let (store, upload, work) = (&search.store, &search.upload, &search.work);
        let count = decide.count as usize;
        let block = self.block(count);
        command.dispatch(
            &self.expand,
            &[
                &store.arena.address,
                &store.offset.memory,
                &upload.entry.memory,
                upload.key.memory(),
                &upload.lone.memory,
                &upload.size.memory,
                &upload.base.memory,
                &work.start.memory,
                &work.record.memory,
                &work.sum.memory,
                &work.event.memory,
                &work.summary,
                upload.catalog.memory(),
                &upload.part.memory,
                &upload.member.memory,
                &upload.coherence.memory,
                &upload.span.memory,
                &upload.reach.memory,
                &upload.rule.memory,
                &upload.arity.memory,
                &work.route.memory,
            ],
            &setting.byte(),
            [self.group(setting.count as usize) * self.width, 1, 1],
            [self.width, 1, 1],
        )?;
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
        )?;
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
            [block * self.span, 1, 1],
            [self.span, 1, 1],
        )?;
        self.scan.encode(
            command,
            &work.block.memory,
            block,
            &work.total,
            WINNER,
            &work.level,
        )?;
        if decide.allowed < decide.count {
            let bound = Setting {
                count: saturate(block),
                allowed: decide.allowed,
                ..Setting::default()
            };
            command.dispatch(
                &self.bound,
                &[&work.block.memory, &work.total],
                &bound.byte(),
                [block + 1, 1, 1],
                self.bound.group([block + 1, 1, 1]),
            )?;
        }
        Ok(())
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
        let block = self.block(count);
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
            [block * self.span, 1, 1],
            [self.span, 1, 1],
        )?;
        command.dispatch(
            &self.point,
            &[
                &work.target.memory,
                &store.table,
                &work.slot.memory,
                &work.total,
            ],
            &decide.byte(),
            [count, 1, 1],
            self.point.group([count, 1, 1]),
        )?;
        if decide.allowed < decide.count {
            command.dispatch(
                &self.weigh,
                &[
                    &upload.entry.memory,
                    &work.record.memory,
                    &work.target.memory,
                    &work.event.memory,
                    &work.summary,
                    &work.total,
                ],
                &decide.byte(),
                [block * self.span, 1, 1],
                [self.span, 1, 1],
            )?;
        }
        if let Some(setting) = later {
            self.census(command, search, &setting)?;
        }
        Ok(())
    }

    // Keeps the pass's new markings in the arena: the command wrote them to its last segment when
    // they fit, and otherwise a second command writes those that fit there and the rest to a new
    // segment.
    fn spill(
        &self,
        search: &mut Search<'_>,
        decide: &Setting,
        later: Option<Setting>,
    ) -> Result<(), Failure> {
        let total = search.work.total.view::<u64>();
        let word = total[BOUND].min(total[WINNER] & WORD);
        if word <= decide.room {
            search.store.arena.advance(word as usize);
            return Ok(());
        }
        let kept = {
            let block = self.block(decide.count as usize);
            let prefix = &search.work.block.memory.view::<u64>()[..block];
            let last = prefix[1..].partition_point(|&value| value & WORD <= decide.room);
            prefix[last] & WORD
        };
        let overflow = search
            .store
            .arena
            .split(&self.device, kept, word as usize)?;
        let decide = Setting {
            split: kept,
            overflow,
            room: u64::MAX,
            ..*decide
        };
        let mut command = self.command(&search.store.arena)?;
        self.settle(&mut command, search, &decide, later)?;
        command.run()?;
        Ok(())
    }

    // Adds up what a decided pass found: its events, those the kernels summed a threadgroup at a
    // time and those of the successors the host found that reached a marking, the events of the
    // host's successors the limits refused, every candidate's target when cycles matter, and the
    // new markings the configuration limit admits.
    fn account(&self, search: &mut Search<'_>, joined: &[Joined], range: &Range, allowed: usize) {
        let count = range.count();
        let weighed = if allowed < count {
            self.block(count)
        } else {
            self.group(range.to - range.from)
        };
        let summed = search.work.event.memory.view::<u64>()[..weighed]
            .iter()
            .sum::<u64>();
        let target = search.work.target.memory.view::<u32>();
        let mut found = 0;
        for item in joined {
            let weight = usize::try_from(item.weight).unwrap_or(usize::MAX);
            match item.refused {
                Some(bound) => search.tally.blocked.add(bound, weight),
                None if target[(item.position - range.begin) as usize] == BLOCKED => {
                    search.tally.blocked.add(Bound::Configuration, weight);
                }
                None => found += item.weight,
            }
        }
        search.tally.event += summed + found;
        if search.cycle == Cycle::Find {
            search.tally.link(&target[..count]);
        }
        let winner = (search.work.total.view::<u64>()[WINNER] >> SHIFT) as usize;
        search.store.count += winner.min(allowed);
        search.store.fresh = winner as f64 / count as f64;
    }
}
