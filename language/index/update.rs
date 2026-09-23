use super::Index;
use crate::basis::Set;
use crate::change::Change;
use crate::location::Location;
use crate::profile;
use crate::state::State;
use crate::term::Term;
use smallvec::SmallVec;
use std::ops::Range;
use std::sync::Arc;

struct Presence {
    frame: usize,
    present: bool,
}

impl Index {
    #[cfg(any(test, feature = "measurement"))]
    pub(crate) fn update(&mut self, state: Arc<State>, change: &Change) {
        let reach = self.reach.advance(&self.state, &state, change);
        self.apply(state, change, reach);
    }

    pub(crate) fn apply(
        &mut self,
        state: Arc<State>,
        change: &Change,
        reach: crate::reachability::Index,
    ) {
        let _scope = profile::Scope::new(profile::Phase::Index);
        self.previous = self.revision.take();
        self.delta.clear();
        let context = self.contextual(&state, change, &reach);
        self.delta.invalidated = context.invalidated;
        let affected = self.presence(&state, change);
        let posting = self.withdraw(&change.world);
        self.prune(&affected, posting);
        self.renumber();
        self.invalidate();
        let replacement = self.replace(&state, &reach, &context.repopulated);
        self.install(state, reach);
        self.admit(replacement, change.insertion.clone());
        self.seal(affected);
        self.delta.repopulated = context.repopulated;
    }

    fn presence(&self, state: &State, change: &Change) -> Vec<Presence> {
        let mut frame = change
            .world
            .iter()
            .map(|&world| self.state.world[world].frame)
            .chain(
                state
                    .world
                    .range(change.insertion.clone())
                    .map(|world| world.frame),
            )
            .collect::<Vec<_>>();
        frame.sort_unstable();
        frame.dedup();
        frame
            .into_iter()
            .map(|frame| Presence {
                frame,
                present: self.present(frame),
            })
            .collect()
    }

    fn withdraw(&mut self, removed: &Set<usize>) -> SmallVec<[(usize, Term); 8]> {
        self.delta
            .removal
            .extend(removed.iter().map(|&world| self.coherence[world]));
        let mut posting = SmallVec::new();
        let prior = self.state.clone();
        for (offset, &world) in removed.iter().enumerate() {
            let site = self.delta.removal[offset];
            let value = &prior.world[world];
            self.delta
                .affected
                .insert(value.frame, value.particle.iter().map(|token| token.value));
            self.frame[value.frame].remove(&site);
            self.retained -= 1;
            for token in &value.particle {
                self.release(token.value);
                posting.push((value.frame, Term::new(token.value, token.capture)));
            }
            self.retire(site);
        }
        posting
    }

    fn prune(&mut self, affected: &[Presence], mut posting: SmallVec<[(usize, Term); 8]>) {
        for presence in affected {
            let Some(reader) = self.reader.get_mut(presence.frame) else {
                continue;
            };
            let previous = reader.len();
            reader.retain(|reader| self.location[reader.site].is_some());
            self.retained -= previous - reader.len();
        }
        posting.sort_unstable();
        posting.dedup();
        for key in posting {
            let posting = self.term.get_mut(&key).unwrap();
            let previous = posting.len();
            posting.retain(|occurrence| self.location[occurrence.site].is_some());
            self.retained -= previous - posting.len();
            if posting.is_empty() {
                self.term.remove(&key);
            }
        }
    }

    fn renumber(&mut self) {
        self.position.compact(&mut self.rank);
        self.coherence.retain(|&site| self.location[site].is_some());
        for (world, &site) in self.coherence.iter().enumerate() {
            self.location[site] = Some(Location::World(world));
        }
    }

    fn invalidate(&mut self) {
        for &frame in &self.delta.invalidated {
            if let Some(site) = self.owner.get(frame).copied().flatten() {
                self.delta.removal.push(site);
            }
            if let Some(member) = self.frame.get(frame) {
                self.delta.removal.extend(member.iter().copied());
            }
        }
    }

    fn replace(
        &mut self,
        state: &State,
        reach: &crate::reachability::Index,
        repopulated: &[usize],
    ) -> Vec<usize> {
        let mut replacement = Vec::new();
        for &frame in repopulated {
            if self.present(frame)
                && reach.frame.binary_search(&frame).is_ok()
                && self.state.frame[frame]
                    .particle
                    .shared(&state.frame[frame].particle)
            {
                self.reconcile(frame, &state.frame[frame].particle);
                continue;
            }
            self.detach(frame, reach.frame.binary_search(&frame).is_err());
            replacement.push(frame);
        }
        replacement
    }

    fn install(&mut self, state: Arc<State>, reach: crate::reachability::Index) {
        self.state = state;
        self.reach = reach;
        self.owner.resize(self.state.frame.len(), None);
        self.frame
            .resize_with(self.state.frame.len(), crate::membership::Set::default);
        self.reader.resize_with(self.state.frame.len(), Vec::new);
    }

    fn admit(&mut self, replacement: Vec<usize>, insertion: Range<usize>) {
        for frame in replacement {
            if self.present(frame) {
                self.attach(frame);
            }
        }
        for world in insertion {
            self.insert(world);
        }
        for &frame in &self.delta.invalidated {
            if !self.present(frame) {
                continue;
            }
            self.delta
                .insertion
                .extend(self.frame[frame].iter().copied());
            self.delta.insertion.extend(self.owner[frame]);
        }
    }

    fn seal(&mut self, affected: Vec<Presence>) {
        self.delta.removal.sort_unstable();
        self.delta.removal.dedup();
        if !self.delta.invalidated.is_empty() {
            let location = &self.location;
            self.delta
                .insertion
                .sort_unstable_by_key(|&site| location[site]);
            self.delta.insertion.dedup();
        }
        for presence in affected {
            if presence.present != self.present(presence.frame) {
                self.delta.invalidated.push(presence.frame);
            }
        }
        self.delta.invalidated.sort_unstable();
        self.delta.invalidated.dedup();
        self.delta.affected.seal();
    }
}
