#[cfg(test)]
use crate::basis::Set;
use crate::program::Symbol;
use crate::state::State;
use crate::term::Term;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

mod posting;

use posting::Occurrence;

pub(crate) struct Index {
    pub state: Arc<State>,
    revision: OnceLock<Arc<()>>,
    previous: Option<Arc<()>>,
    frame: Vec<crate::membership::Set>,
    reader: Vec<Vec<crate::reader::Reader>>,
    term: HashMap<(usize, Term), Vec<Occurrence>>,
    position: crate::position::Index,
    rank: Vec<usize>,
    vacant: Vec<usize>,
    retained: usize,
    symbol: HashMap<Symbol, usize>,
    pub altered: std::collections::HashSet<Symbol>,
    pub context: Vec<usize>,
    pub affected: HashMap<usize, std::collections::HashSet<Symbol>>,
    pub removal: Vec<usize>,
    pub insertion: Vec<usize>,
}

impl Index {
    pub fn new(state: Arc<State>) -> Self {
        let mut index = Self {
            state,
            revision: OnceLock::new(),
            previous: None,
            frame: Vec::new(),
            reader: Vec::new(),
            term: HashMap::new(),
            position: Default::default(),
            rank: Vec::new(),
            vacant: Vec::new(),
            retained: 0,
            symbol: HashMap::new(),
            altered: Default::default(),
            context: Vec::new(),
            affected: HashMap::new(),
            removal: Vec::new(),
            insertion: Vec::new(),
        };
        index
            .frame
            .resize_with(index.state.frame.len(), crate::membership::Set::default);
        index.reader.resize_with(index.state.frame.len(), Vec::new);
        for world in 0..index.state.world.len() {
            index.insert(world);
        }
        index
    }

    pub fn revision(&self) -> &Arc<()> {
        self.revision.get_or_init(|| Arc::new(()))
    }

    pub fn previous(&self) -> Option<&Arc<()>> {
        self.previous.as_ref()
    }

    fn insert(&mut self, world: usize) {
        let site = self.vacant.pop().unwrap_or_else(|| {
            let site = self.rank.len();
            self.rank.push(0);
            site
        });
        self.rank[site] = self.position.insert(site);
        self.insertion.push(site);
        let value = &self.state.world[world];
        self.affected
            .entry(value.frame)
            .or_default()
            .extend(value.particle.iter().map(|token| token.value));
        self.frame[value.frame].insert(site);
        self.retained += 1;
        for token in &value.particle {
            if let Symbol::Rule(rule) = token.value {
                self.reader[value.frame].push(crate::reader::Reader {
                    rule,
                    read: crate::reader::Read {
                        site,
                        resource: token.id,
                    },
                    owner: token.capture.unwrap(),
                });
                self.retained += 1;
            }
            let count = self.symbol.entry(token.value).or_default();
            if *count == 0 && !self.altered.remove(&token.value) {
                self.altered.insert(token.value);
            }
            *count += 1;
            let key = (value.frame, Term::new(token.value, token.capture));
            let posting = self.term.entry(key).or_default();
            if let Some(occurrence) = posting
                .last_mut()
                .filter(|occurrence| occurrence.site == site)
            {
                occurrence.count += 1;
            } else {
                posting.push(Occurrence { site, count: 1 });
                self.retained += 1;
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn advance(&mut self, state: Arc<State>, removed: &Set<usize>) {
        let change = crate::change::Change {
            world: removed.clone(),
            insertion: self.state.world.len() - removed.len()..state.world.len(),
            frame: Vec::new(),
        };
        self.update(state, &change);
    }

    pub(crate) fn update(&mut self, state: Arc<State>, change: &crate::change::Change) {
        #[cfg(feature = "measurement")]
        let _measurement =
            crate::measurement::profile::Scope::new(crate::measurement::profile::Phase::Index);
        self.previous = self.revision.take();
        let removed = &change.world;
        self.altered.clear();
        self.affected.clear();
        self.removal.clear();
        self.insertion.clear();
        self.context.clear();
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
            .extend(removed.iter().map(|&world| self.position.select(world)));
        let mut posting = smallvec::SmallVec::<[(usize, Term); 8]>::new();
        for (&world, &site) in removed.iter().zip(&self.removal) {
            let value = &self.state.world[world];
            self.affected
                .entry(value.frame)
                .or_default()
                .extend(value.particle.iter().map(|token| token.value));
            self.frame[value.frame].remove(&site);
            self.retained -= 1;
            for token in &value.particle {
                let count = self.symbol.get_mut(&token.value).unwrap();
                *count -= 1;
                if *count == 0 {
                    self.symbol.remove(&token.value);
                    self.altered.insert(token.value);
                }
                posting.push((value.frame, Term::new(token.value, token.capture)));
            }
            self.position.remove(self.rank[site]);
            self.rank[site] = usize::MAX;
            self.vacant.push(site);
        }
        for &frame in &affected {
            let Some(reader) = self.reader.get_mut(frame) else {
                continue;
            };
            let previous = reader.len();
            reader.retain(|reader| self.rank[reader.read.site] != usize::MAX);
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
        self.state = state;
        self.frame
            .resize_with(self.state.frame.len(), crate::membership::Set::default);
        self.reader.resize_with(self.state.frame.len(), Vec::new);
        for world in change.insertion.clone() {
            self.insert(world);
        }
        for (frame, previous) in affected.into_iter().zip(previous) {
            if previous != self.present(frame) {
                self.context.push(frame);
            }
        }
    }

    pub fn candidate(&self, pattern: &[Term], frame: usize) -> Vec<usize> {
        let Some(world) = self.frame.get(frame) else {
            return Vec::new();
        };
        if pattern.is_empty() {
            let mut candidate = world
                .iter()
                .map(|&site| self.world(site))
                .collect::<Vec<_>>();
            candidate.sort_unstable();
            return candidate;
        }
        if let [term] = pattern {
            return self
                .posting(term, frame)
                .into_iter()
                .flatten()
                .map(|occurrence| self.world(occurrence.site))
                .collect();
        }
        let posting = pattern
            .iter()
            .map(|term| self.posting(term, frame))
            .collect::<Option<Vec<_>>>();
        let Some(posting) = posting else {
            return Vec::new();
        };
        posting::intersect(&posting, &self.rank)
            .map(|site| self.world(site))
            .collect()
    }

    pub(crate) fn quantity(&self, term: &Term, frame: usize, site: usize) -> usize {
        let Some(posting) = self.posting(term, frame) else {
            return 0;
        };
        posting
            .binary_search_by_key(&self.rank[site], |occurrence| self.rank[occurrence.site])
            .map_or(0, |position| posting[position].count)
    }

    pub(crate) fn site(&self, world: usize) -> usize {
        self.position.select(world)
    }

    fn posting(&self, term: &Term, frame: usize) -> Option<&[Occurrence]> {
        self.term
            .get(&(frame, Term::new(term.value, term.capture)))
            .map(Vec::as_slice)
    }

    pub(crate) fn precedes(&self, left: usize, right: usize) -> bool {
        self.rank[left] < self.rank[right]
    }

    pub(crate) fn world(&self, site: usize) -> usize {
        self.position.rank(self.rank[site])
    }

    pub fn retained(&self) -> usize {
        self.frame.len()
            + self.reader.len()
            + self.term.len()
            + self.retained
            + self.position.retained()
            + self.rank.len()
            + self.vacant.len()
            + self.symbol.len()
            + self.context.len()
            + self.affected.len()
            + self
                .affected
                .values()
                .map(std::collections::HashSet::len)
                .sum::<usize>()
            + self.altered.len()
            + self.removal.len()
            + self.insertion.len()
    }

    pub(crate) fn reader(&self, frame: usize) -> &[crate::reader::Reader] {
        self.reader.get(frame).map_or(&[], Vec::as_slice)
    }

    pub(crate) fn present(&self, frame: usize) -> bool {
        frame < self.state.frame.len()
            && (frame == 0 || self.frame.get(frame).is_some_and(|world| !world.is_empty()))
    }

    pub(crate) fn frame(&self) -> impl Iterator<Item = usize> + '_ {
        self.frame
            .iter()
            .enumerate()
            .filter_map(|(index, world)| (index == 0 || !world.is_empty()).then_some(index))
    }

    pub(crate) fn available(&self) -> impl Iterator<Item = Symbol> + '_ {
        self.symbol.keys().copied()
    }

    pub(crate) fn contains(&self, symbol: &Symbol) -> bool {
        self.symbol.contains_key(symbol)
    }
}

#[cfg(test)]
#[path = "test/index.rs"]
mod test;
