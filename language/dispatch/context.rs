use crate::index::Index;
use crate::membership::Set;
use crate::state::State;
use smallvec::SmallVec;

fn scan(index: &Index, changed: &[usize]) -> SmallVec<[usize; 4]> {
    let mut known = vec![None; index.state.frame.len()];
    for &frame in changed {
        if let Some(value) = known.get_mut(frame) {
            *value = Some(true);
        }
    }
    let mut selected = SmallVec::new();
    let mut path = SmallVec::<[usize; 8]>::new();
    for frame in index.frame() {
        let mut owner = Some(frame);
        let affected = loop {
            let Some(current) = owner else {
                break false;
            };
            if let Some(affected) = known[current] {
                break affected;
            }
            path.push(current);
            owner = index.state.frame[current].lexical;
        };
        for current in path.drain(..) {
            known[current] = Some(affected);
        }
        if affected {
            selected.push(frame);
        }
    }
    selected
}

#[derive(Default)]
pub(super) struct Context {
    forest: Option<Forest>,
    disabled: bool,
}

struct Forest {
    child: Vec<Set>,
    retained: usize,
}

impl Context {
    pub fn select(
        &mut self,
        index: &Index,
        previous: &State,
        changed: &[usize],
    ) -> SmallVec<[usize; 4]> {
        #[cfg(feature = "measurement")]
        let _measurement =
            crate::measurement::profile::Scope::new(crate::measurement::profile::Phase::Context);
        if self.disabled || index.state.frame.len() < 32 {
            self.forest = None;
            return scan(index, changed);
        }
        if let Some(forest) = &mut self.forest {
            forest.update(&index.state, previous, changed);
        } else {
            self.forest = Some(Forest::new(&index.state));
        }
        self.forest.as_ref().unwrap().select(index, changed)
    }

    pub fn evict(&mut self) {
        self.forest = None;
        self.disabled = true;
    }

    pub fn retained(&self) -> usize {
        self.forest
            .as_ref()
            .map_or(0, |forest| forest.child.len() + forest.retained)
    }
}

impl Forest {
    fn new(state: &State) -> Self {
        let mut forest = Self {
            child: vec![Set::default(); state.frame.len()],
            retained: 0,
        };
        for (frame, value) in state.frame.iter().enumerate() {
            if let Some(parent) = value.lexical {
                forest.child[parent].insert(frame);
                forest.retained += 1;
            }
        }
        forest
    }

    fn update(&mut self, state: &State, previous: &State, changed: &[usize]) {
        self.child
            .resize_with(self.child.len().max(state.frame.len()), Set::default);
        for &frame in changed {
            if let Some(parent) = previous.frame.get(frame).and_then(|value| value.lexical) {
                self.retained -= usize::from(self.child[parent].remove(&frame));
            }
        }
        for &frame in changed {
            if let Some(parent) = state.frame.get(frame).and_then(|value| value.lexical) {
                self.retained += usize::from(self.child[parent].insert(frame));
            }
        }
    }

    fn select(&self, index: &Index, changed: &[usize]) -> SmallVec<[usize; 4]> {
        let mut pending = SmallVec::<[usize; 8]>::from_slice(changed);
        let mut visited = Set::default();
        let mut selected = SmallVec::new();
        while let Some(frame) = pending.pop() {
            if !visited.insert(frame) {
                continue;
            }
            if index.present(frame) {
                selected.push(frame);
            }
            if let Some(child) = self.child.get(frame) {
                pending.extend(child.iter().copied());
            }
        }
        selected.sort_unstable();
        selected
    }
}

#[cfg(test)]
#[path = "../test/context.rs"]
mod test;
