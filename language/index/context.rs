use super::Index;
use crate::change::Change;
use crate::location::Location;
use crate::state::State;

pub(super) struct Context {
    pub frame: Vec<usize>,
    pub affected: Vec<usize>,
}

impl Index {
    pub(super) fn contextual(
        &mut self,
        state: &State,
        change: &Change,
        reach: &crate::reachability::Index,
    ) -> Context {
        #[cfg(feature = "measurement")]
        let _measurement =
            crate::measurement::profile::Scope::new(crate::measurement::profile::Phase::Context);
        let changed = change
            .frame
            .iter()
            .copied()
            .filter(
                |&frame| match (self.state.frame.get(frame), state.frame.get(frame)) {
                    (Some(left), Some(right)) => {
                        left.scope != right.scope
                            || left.lexical != right.lexical
                            || left.particle != right.particle
                    }
                    _ => true,
                },
            )
            .collect::<Vec<_>>();
        let mut selected = self.lexical.select(&changed);
        self.lexical.update(&self.state, state, &change.frame);
        selected.extend(self.lexical.select(&changed));
        let mut frame = change
            .frame
            .iter()
            .copied()
            .filter(
                |&frame| match (self.state.frame.get(frame), state.frame.get(frame)) {
                    (Some(left), Some(right)) => {
                        !left.particle.shared(&right.particle) || left.particle != right.particle
                    }
                    _ => true,
                },
            )
            .collect::<Vec<_>>();
        if self.reach.frame != reach.frame {
            let changed = self
                .reach
                .frame
                .iter()
                .filter(|frame| reach.frame.binary_search(frame).is_err())
                .chain(
                    reach
                        .frame
                        .iter()
                        .filter(|frame| self.reach.frame.binary_search(frame).is_err()),
                )
                .copied();
            frame.extend(changed.clone());
            selected.extend(changed);
        }
        frame.sort_unstable();
        frame.dedup();
        selected.sort_unstable();
        selected.dedup();
        Context {
            frame,
            affected: selected,
        }
    }

    pub(super) fn detach(&mut self, frame: usize, retired: bool) {
        let Some(site) = self.owner.get(frame).copied().flatten() else {
            return;
        };
        for token in &self.state.frame[frame].particle {
            self.retained -= 1;
            let count = self.symbol.get_mut(&token.value).unwrap();
            *count -= 1;
            if *count == 0 {
                self.symbol.remove(&token.value);
                if !self.altered.remove(&token.value) {
                    self.altered.insert(token.value);
                }
            }
            self.lexicon.remove(&(frame, token.value));
        }
        if retired {
            self.position.remove(self.rank[site]);
            self.rank[site] = usize::MAX;
            self.location[site] = None;
            self.vacant.push(site);
            self.owner[frame] = None;
        }
    }

    pub(super) fn attach(&mut self, frame: usize) {
        if self.owner[frame].is_none() {
            let site = self.allocate(Location::Context(frame));
            self.owner[frame] = Some(site);
        }
        for (position, token) in self.state.frame[frame].particle.entry() {
            self.retained += 1;
            let count = self.symbol.entry(token.value).or_default();
            if *count == 0 && !self.altered.remove(&token.value) {
                self.altered.insert(token.value);
            }
            *count += 1;
            self.lexicon
                .entry((frame, token.value))
                .or_default()
                .push(position);
        }
    }
}

impl Index {
    pub(super) fn reconcile(&mut self, frame: usize, current: &crate::population::Set) {
        let previous = &self.state.frame[frame].particle;
        for position in previous.removed(current) {
            let value = previous.at(position).value;
            self.retained -= 1;
            let count = self.symbol.get_mut(&value).unwrap();
            *count -= 1;
            if *count == 0 {
                self.symbol.remove(&value);
                if !self.altered.remove(&value) {
                    self.altered.insert(value);
                }
            }
            let key = (frame, value);
            let occurrence = self.lexicon.get_mut(&key).unwrap();
            let offset = occurrence.binary_search(&position).unwrap();
            occurrence.remove(offset);
            if occurrence.is_empty() {
                self.lexicon.remove(&key);
            }
        }
        for position in current.removed(previous) {
            let value = current.at(position).value;
            self.retained += 1;
            let count = self.symbol.entry(value).or_default();
            if *count == 0 && !self.altered.remove(&value) {
                self.altered.insert(value);
            }
            *count += 1;
            let occurrence = self.lexicon.entry((frame, value)).or_default();
            let offset = occurrence.binary_search(&position).unwrap_err();
            occurrence.insert(offset, position);
        }
    }
}
