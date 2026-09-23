use super::Index;
use crate::location::Location;
use crate::profile;
use crate::state::State;
use crate::term::Term;
use std::sync::Arc;

impl Index {
    #[cfg(any(test, feature = "measurement"))]
    pub(crate) fn update(&mut self, state: Arc<State>, change: &crate::change::Change) {
        let reach = self.reach.advance(&self.state, &state, change);
        self.apply(state, change, reach);
    }

    pub(crate) fn apply(
        &mut self,
        state: Arc<State>,
        change: &crate::change::Change,
        reach: crate::reachability::Index,
    ) {
        let _scope = profile::Scope::new(profile::Phase::Index);
        self.previous = self.revision.take();
        let removed = &change.world;
        self.delta.clear();
        let context = self.contextual(&state, change, &reach);
        self.delta.invalidated = context.invalidated;
        let mut affected = change
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
        affected.sort_unstable();
        affected.dedup();
        let previous = affected
            .iter()
            .map(|&frame| self.present(frame))
            .collect::<Vec<_>>();
        self.delta
            .removal
            .extend(removed.iter().map(|&world| self.coherence[world]));
        let mut posting = smallvec::SmallVec::<[(usize, Term); 8]>::new();
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
        for &frame in &affected {
            let Some(reader) = self.reader.get_mut(frame) else {
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
        self.position.compact(&mut self.rank);
        self.coherence.retain(|&site| self.location[site].is_some());
        for (world, &site) in self.coherence.iter().enumerate() {
            self.location[site] = Some(Location::World(world));
        }
        for &frame in &self.delta.invalidated {
            if let Some(site) = self.owner.get(frame).copied().flatten() {
                self.delta.removal.push(site);
            }
            if let Some(member) = self.frame.get(frame) {
                self.delta.removal.extend(member.iter().copied());
            }
        }
        let mut replacement = Vec::new();
        for &frame in &context.repopulated {
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
        self.state = state;
        self.reach = reach;
        self.owner.resize(self.state.frame.len(), None);
        self.frame
            .resize_with(self.state.frame.len(), crate::membership::Set::default);
        self.reader.resize_with(self.state.frame.len(), Vec::new);
        for frame in replacement {
            if self.present(frame) {
                self.attach(frame);
            }
        }
        for world in change.insertion.clone() {
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
        self.delta.removal.sort_unstable();
        self.delta.removal.dedup();
        if !self.delta.invalidated.is_empty() {
            let location = &self.location;
            self.delta
                .insertion
                .sort_unstable_by_key(|&site| location[site]);
            self.delta.insertion.dedup();
        }
        for (frame, previous) in affected.into_iter().zip(previous) {
            if previous != self.present(frame) {
                self.delta.invalidated.push(frame);
            }
        }
        self.delta.invalidated.sort_unstable();
        self.delta.invalidated.dedup();
        self.delta.affected.seal();
        self.delta.repopulated = context.repopulated;
    }
}
