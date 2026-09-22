#[cfg(test)]
use crate::basis::Set;
use crate::location::Location;
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
    reach: crate::reachability::Index,
    location: Vec<Option<Location>>,
    coherence: Vec<usize>,
    owner: Vec<Option<usize>>,
    lexicon: HashMap<(usize, Symbol), Vec<usize>>,
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
            reach: crate::reachability::Index::new(&state),
            state,
            revision: OnceLock::new(),
            previous: None,
            frame: Vec::new(),
            location: Vec::new(),
            coherence: Vec::new(),
            owner: Vec::new(),
            lexicon: HashMap::new(),
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
        index.owner.resize(index.state.frame.len(), None);
        for &frame in index.reach.frame.clone().iter() {
            let site = index.allocate(Location::Context(frame));
            index.owner[frame] = Some(site);
            let value = index.state.frame[frame].clone();
            for (position, token) in value.particle.iter().enumerate() {
                *index.symbol.entry(token.value).or_default() += 1;
                index
                    .lexicon
                    .entry((frame, token.value))
                    .or_default()
                    .push(position);
            }
        }
        for &frame in index.reach.frame.clone().iter() {
            index
                .affected
                .entry(frame)
                .or_default()
                .extend(index.state.visible(frame).map(|(_, token)| token.value));
        }
        index
    }

    pub fn revision(&self) -> &Arc<()> {
        self.revision.get_or_init(|| Arc::new(()))
    }

    pub fn previous(&self) -> Option<&Arc<()>> {
        self.previous.as_ref()
    }

    fn allocate(&mut self, location: Location) -> usize {
        let site = self.vacant.pop().unwrap_or_else(|| {
            let site = self.rank.len();
            self.rank.push(0);
            self.location.push(None);
            site
        });
        self.location[site] = Some(location);
        self.rank[site] = self.position.insert(site);
        self.insertion.push(site);
        site
    }

    fn insert(&mut self, world: usize) {
        let site = self.allocate(Location::World(world));
        self.coherence.push(site);
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
                    read: crate::reader::Read::World(site, token.id),
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
        let contextual = !change.frame.is_empty()
            || self.state.frame.len() != state.frame.len()
            || self
                .state
                .frame
                .iter()
                .zip(state.frame.iter())
                .any(|(left, right)| !Arc::ptr_eq(left, right) && left != right);
        let reach = (!contextual).then(|| self.reach.advance(&self.state, &state, change));
        if contextual
            || reach
                .as_ref()
                .is_some_and(|reach| reach.frame != self.reach.frame)
        {
            let mut fresh = Self::new(state);
            fresh.previous = self.revision.take();
            fresh.removal = self
                .location
                .iter()
                .enumerate()
                .filter_map(|(site, location)| location.is_some().then_some(site))
                .collect();
            fresh.altered = self
                .symbol
                .keys()
                .chain(fresh.symbol.keys())
                .copied()
                .filter(|symbol| {
                    self.symbol.contains_key(symbol) != fresh.symbol.contains_key(symbol)
                })
                .collect();
            fresh.context = self
                .reach
                .frame
                .iter()
                .chain(fresh.reach.frame.iter())
                .copied()
                .collect();
            fresh.context.sort_unstable();
            fresh.context.dedup();
            *self = fresh;
            return;
        }
        self.reach = reach.unwrap();
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
            .extend(removed.iter().map(|&world| self.coherence[world]));
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

    pub fn candidate(&self, pattern: impl IntoIterator<Item = Term>, frame: usize) -> Vec<usize> {
        let pattern = pattern.into_iter().collect::<Vec<_>>();
        let mut posting = smallvec::SmallVec::<[&[Occurrence]; 4]>::new();
        for term in &pattern {
            if self.visible(frame, term).next().is_some() {
                continue;
            }
            let Some(value) = self.posting(term, frame) else {
                return Vec::new();
            };
            posting.push(value);
        }
        if !posting.is_empty() {
            return posting::intersect(&posting, &self.rank).collect();
        }
        let mut candidate = self
            .frame
            .get(frame)
            .into_iter()
            .flat_map(|frame| frame.iter().copied())
            .collect::<Vec<_>>();
        if !pattern.is_empty()
            && let Some(site) = self.owner.get(frame).copied().flatten()
        {
            candidate.push(site);
        }
        candidate.sort_unstable_by_key(|&site| self.location(site));
        candidate
    }

    pub(crate) fn occurrence(
        &self,
        frame: usize,
        symbol: Symbol,
    ) -> impl Iterator<Item = (crate::flow::Place, &crate::state::Token)> {
        std::iter::successors(self.state.frame.get(frame).map(|_| frame), |&frame| {
            self.state.frame[frame].lexical
        })
        .flat_map(move |frame| {
            self.lexicon
                .get(&(frame, symbol))
                .into_iter()
                .flatten()
                .map(move |&position| {
                    let token = &self.state.frame[frame].particle[position];
                    (crate::flow::Place::Context(frame, token.id), token)
                })
        })
    }

    pub(crate) fn visible<'index>(
        &'index self,
        frame: usize,
        term: &'index Term,
    ) -> impl Iterator<Item = &'index crate::state::Token> {
        self.occurrence(frame, term.value)
            .map(|(_, token)| token)
            .filter(|token| term.matches(token))
    }

    pub(crate) fn particle(
        &self,
        site: usize,
        pattern: impl IntoIterator<Item = Term>,
    ) -> Vec<crate::state::Token> {
        let location = self.location(site);
        let frame = location.frame(&self.state);
        let mut particle = location
            .world()
            .map_or_else(Vec::new, |world| self.state.world[world].particle.clone());
        let mut pattern = pattern.into_iter().collect::<Vec<_>>();
        pattern.sort_unstable();
        pattern.dedup();
        for term in pattern {
            particle.extend(self.visible(frame, &term).cloned());
        }
        particle
    }

    pub(crate) fn quantity(&self, term: &Term, frame: usize, site: usize) -> usize {
        let local = self.posting(term, frame).map_or(0, |posting| {
            posting
                .binary_search_by_key(&self.rank[site], |occurrence| self.rank[occurrence.site])
                .map_or(0, |position| posting[position].count)
        });
        local + self.visible(frame, term).count()
    }

    pub(crate) fn site(&self, world: usize) -> usize {
        self.coherence[world]
    }

    pub(crate) fn locate(&self, location: Location) -> usize {
        match location {
            Location::World(world) => self.site(world),
            Location::Context(frame) => self.owner[frame].unwrap(),
        }
    }

    pub(crate) fn location(&self, site: usize) -> Location {
        self.location[site].unwrap()
    }

    fn posting(&self, term: &Term, frame: usize) -> Option<&[Occurrence]> {
        self.term
            .get(&(frame, Term::new(term.value, term.capture)))
            .map(Vec::as_slice)
    }

    pub(crate) fn precedes(&self, left: usize, right: usize) -> bool {
        self.location(left) < self.location(right)
    }

    pub(crate) fn world(&self, site: usize) -> usize {
        self.location(site).world().expect("coherence location")
    }

    pub fn retained(&self) -> usize {
        self.frame.len()
            + self.reach.retained()
            + self.location.len()
            + self.coherence.len()
            + self.owner.len()
            + self.lexicon.len()
            + self.lexicon.values().map(Vec::len).sum::<usize>()
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
        self.reach.frame.binary_search(&frame).is_ok()
    }

    pub(crate) fn frame(&self) -> impl Iterator<Item = usize> + '_ {
        self.reach.frame.iter().copied()
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
