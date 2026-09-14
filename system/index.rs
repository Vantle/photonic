use crate::matching::Term;
use crate::program::Symbol;
use crate::state::State;
use std::collections::HashMap;
use std::sync::Arc;

pub(crate) struct Index {
    pub state: Arc<State>,
    frame: Vec<Vec<usize>>,
    term: HashMap<(usize, Term), Vec<usize>>,
    retained: usize,
}

impl Index {
    pub fn new(state: Arc<State>) -> Self {
        let mut frame = vec![Vec::new(); state.frame.len()];
        let mut term = HashMap::<_, Vec<usize>>::new();
        for (site, world) in state.world.iter().enumerate() {
            frame[world.frame].push(site);
            for token in &world.particle {
                let key = Term {
                    value: token.value,
                    capture: token
                        .capture
                        .filter(|_| matches!(token.value, Symbol::Rule(_))),
                };
                let posting = term.entry((world.frame, key)).or_default();
                if posting.last() != Some(&site) {
                    posting.push(site);
                }
            }
        }
        let retained = frame.len()
            + frame.iter().map(Vec::len).sum::<usize>()
            + term.len()
            + term.values().map(Vec::len).sum::<usize>();
        Self {
            state,
            frame,
            term,
            retained,
        }
    }

    pub fn candidate(&self, pattern: &[Term], frame: usize) -> Vec<usize> {
        let Some(world) = self.frame.get(frame) else {
            return Vec::new();
        };
        let posting = pattern
            .iter()
            .map(|term| {
                let key = Term {
                    value: term.value,
                    capture: term
                        .capture
                        .filter(|_| matches!(term.value, Symbol::Rule(_))),
                };
                self.term.get(&(frame, key))
            })
            .collect::<Option<Vec<_>>>();
        let Some(posting) = posting else {
            return Vec::new();
        };
        let candidate = posting
            .iter()
            .copied()
            .min_by_key(|posting| posting.len())
            .unwrap_or(world);
        candidate
            .iter()
            .filter(|world| {
                posting
                    .iter()
                    .all(|posting| posting.binary_search(world).is_ok())
            })
            .copied()
            .collect()
    }

    pub fn retained(&self) -> usize {
        self.retained
    }
}
