use super::Index;
use crate::location::Location;
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
        #[cfg(feature = "measurement")]
        let _measurement =
            crate::measurement::profile::Scope::new(crate::measurement::profile::Phase::Index);
        self.previous = self.revision.take();
        let removed = &change.world;
        self.altered.clear();
        self.affected.clear();
        self.removal.clear();
        self.insertion.clear();
        let context = self.contextual(&state, change, &reach);
        self.context = context.affected;
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
        self.removal
            .extend(removed.iter().map(|&world| self.coherence[world]));
        let mut posting = smallvec::SmallVec::<[(usize, Term); 8]>::new();
        for (offset, &world) in removed.iter().enumerate() {
            let site = self.removal[offset];
            let value = self.state.world[world].clone();
            self.affected
                .entry(value.frame)
                .or_default()
                .extend(value.particle.iter().map(|token| token.value));
            self.frame[value.frame].remove(&site);
            self.retained -= 1;
            for token in &value.particle {
                self.release(token.value);
                posting.push((value.frame, Term::new(token.value, token.capture)));
            }
            self.position.remove(self.rank[site]);
            self.rank[site] = usize::MAX;
            self.location[site] = None;
            self.vacant.push(site);
        }
        for &frame in &affected {
            let Some(reader) = self.reader.get_mut(frame) else {
                continue;
            };
            let previous = reader.len();
            reader.retain(|reader| match reader.read {
                crate::reader::Read::World(site, _) => self.rank[site] != usize::MAX,
                crate::reader::Read::Context(_, _) => unreachable!(),
            });
            self.retained -= previous - reader.len();
        }
        posting.sort_unstable();
        posting.dedup();
        for key in posting {
            let posting = self.term.get_mut(&key).unwrap();
            let previous = posting.len();
            posting.retain(|occurrence| self.rank[occurrence.site] != usize::MAX);
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
        for &frame in &self.context {
            if let Some(site) = self.owner.get(frame).copied().flatten() {
                self.removal.push(site);
            }
            if let Some(member) = self.frame.get(frame) {
                self.removal.extend(member.iter().copied());
            }
        }
        let mut replacement = Vec::new();
        for &frame in &context.frame {
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
        for &frame in &self.context {
            if !self.present(frame) {
                continue;
            }
            self.insertion.extend(self.frame[frame].iter().copied());
            self.insertion.extend(self.owner[frame]);
        }
        self.removal.sort_unstable();
        self.removal.dedup();
        if !self.context.is_empty() {
            let location = &self.location;
            self.insertion.sort_unstable_by_key(|&site| location[site]);
            self.insertion.dedup();
        }
        for (frame, previous) in affected.into_iter().zip(previous) {
            if previous != self.present(frame) {
                self.context.push(frame);
            }
        }
        self.context.sort_unstable();
        self.context.dedup();
        self.ownership = context.frame;
    }
}
