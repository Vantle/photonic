#[cfg(test)]
use crate::basis::Set;
use crate::hashing::Builder;
use crate::location::Location;
use crate::program::Symbol;
use crate::state::State;
use crate::term::Term;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};

mod context;
mod posting;
mod update;

use posting::Occurrence;

pub(crate) struct Index {
    pub state: Arc<State>,
    revision: OnceLock<Arc<()>>,
    previous: Option<Arc<()>>,
    frame: Vec<crate::membership::Set>,
    reach: crate::reachability::Index,
    lexical: crate::lexical::Index,
    location: Vec<Option<Location>>,
    coherence: Vec<usize>,
    owner: Vec<Option<usize>>,
    lexicon: HashMap<(usize, Symbol), Vec<usize>, Builder>,
    vocabulary: HashMap<Symbol, usize, Builder>,
    reader: Vec<Vec<crate::reader::Reader>>,
    term: HashMap<(usize, Term), Vec<Occurrence>, Builder>,
    position: crate::position::Index,
    rank: Vec<usize>,
    vacant: Vec<usize>,
    retained: usize,
    symbol: HashMap<Symbol, usize, Builder>,
    pub altered: HashSet<Symbol, Builder>,
    pub context: Vec<usize>,
    pub ownership: Vec<usize>,
    pub affected: HashMap<usize, HashSet<Symbol, Builder>, Builder>,
    pub removal: Vec<usize>,
    pub insertion: Vec<usize>,
}

impl Index {
    pub fn new(state: Arc<State>) -> Self {
        let reach = crate::reachability::Index::new(&state);
        Self::prepared(state, reach)
    }

    pub(crate) fn prepared(state: Arc<State>, reach: crate::reachability::Index) -> Self {
        let mut index = Self {
            reach,
            lexical: crate::lexical::Index::new(&state),
            state,
            revision: OnceLock::new(),
            previous: None,
            frame: Vec::new(),
            location: Vec::new(),
            coherence: Vec::new(),
            owner: Vec::new(),
            lexicon: HashMap::default(),
            vocabulary: HashMap::default(),
            reader: Vec::new(),
            term: HashMap::default(),
            position: Default::default(),
            rank: Vec::new(),
            vacant: Vec::new(),
            retained: 0,
            symbol: HashMap::default(),
            altered: Default::default(),
            context: Vec::new(),
            ownership: Vec::new(),
            affected: HashMap::default(),
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
            index.attach(frame);
        }
        index.context = (*index.reach.frame).clone();
        index
    }

    pub fn revision(&self) -> &Arc<()> {
        self.revision.get_or_init(|| Arc::new(()))
    }

    pub fn previous(&self) -> Option<&Arc<()>> {
        self.previous.as_ref()
    }

    pub(crate) fn invalidated(&self, frame: usize) -> bool {
        self.context.binary_search(&frame).is_ok()
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

    fn acquire(&mut self, value: Symbol) {
        let count = self.symbol.entry(value).or_default();
        *count += 1;
        if *count == 1 && !self.altered.remove(&value) {
            self.altered.insert(value);
        }
    }

    fn release(&mut self, value: Symbol) {
        let count = self.symbol.get_mut(&value).unwrap();
        *count -= 1;
        if *count > 0 {
            return;
        }
        self.symbol.remove(&value);
        if !self.altered.remove(&value) {
            self.altered.insert(value);
        }
    }

    fn insert(&mut self, world: usize) {
        let site = self.allocate(Location::World(world));
        self.coherence.push(site);
        let value = self.state.world[world].clone();
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
            self.acquire(token.value);
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
            frame: (0..self.state.frame.len().max(state.frame.len()))
                .filter(|&frame| self.state.frame.get(frame) != state.frame.get(frame))
                .collect(),
        };
        self.update(state, &change);
    }

    pub fn candidate(&self, pattern: impl IntoIterator<Item = Term>, frame: usize) -> Vec<usize> {
        let mut posting = smallvec::SmallVec::<[&[Occurrence]; 4]>::new();
        let mut empty = true;
        for term in pattern {
            empty = false;
            if self.visible(frame, &term).next().is_some() {
                continue;
            }
            let Some(value) = self.posting(&term, frame) else {
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
        if !empty && let Some(site) = self.owner.get(frame).copied().flatten() {
            candidate.push(site);
        }
        candidate.sort_unstable_by_key(|&site| self.location(site));
        candidate
    }

    pub fn possible(&self, pattern: impl IntoIterator<Item = Term>, frame: usize) -> bool {
        pattern.into_iter().all(|term| {
            self.posting(&term, frame).is_some() || self.visible(frame, &term).next().is_some()
        })
    }

    pub(crate) fn occurrence(
        &self,
        frame: usize,
        symbol: Symbol,
    ) -> impl Iterator<Item = (crate::flow::Place, &crate::state::Token)> {
        let start = (frame < self.state.frame.len() && self.vocabulary.contains_key(&symbol))
            .then_some(frame);
        std::iter::successors(start, |&frame| self.state.frame[frame].lexical).flat_map(
            move |frame| {
                self.local(frame, symbol)
                    .map(move |token| (crate::flow::Place::Context(frame, token.id), token))
            },
        )
    }

    pub(crate) fn local(
        &self,
        frame: usize,
        symbol: Symbol,
    ) -> impl Iterator<Item = &crate::state::Token> {
        self.lexicon
            .get(&(frame, symbol))
            .into_iter()
            .flatten()
            .map(move |&position| self.state.frame[frame].particle.at(position))
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
            + self.lexical.retained()
            + self.location.len()
            + self.coherence.len()
            + self.owner.len()
            + self.lexicon.len()
            + self.reader.len()
            + self.term.len()
            + self.retained
            + self.position.retained()
            + self.rank.len()
            + self.vacant.len()
            + self.symbol.len()
            + self.context.len()
            + self.ownership.len()
            + self.affected.len()
            + self.affected.values().map(HashSet::len).sum::<usize>()
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
