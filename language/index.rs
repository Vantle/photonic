use crate::basis::Set;
use crate::program::Symbol;
use crate::state::State;
use crate::term::Term;
use std::collections::HashMap;
use std::sync::Arc;

pub(crate) struct Index {
    pub state: Arc<State>,
    frame: Vec<Vec<usize>>,
    term: HashMap<(usize, Term), Vec<usize>>,
    site: Vec<usize>,
    rank: Vec<usize>,
    vacant: Vec<usize>,
    retained: usize,
    symbol: HashMap<Symbol, usize>,
    code: HashMap<usize, Vec<(usize, usize, usize)>>,
    change: Vec<(Symbol, bool)>,
    pub altered: std::collections::HashSet<Symbol>,
    pub occupied: bool,
    pub removal: Vec<usize>,
    pub insertion: Vec<usize>,
}

impl Index {
    pub fn new(state: Arc<State>) -> Self {
        let mut index = Self {
            state,
            frame: Vec::new(),
            term: HashMap::new(),
            site: Vec::new(),
            rank: Vec::new(),
            vacant: Vec::new(),
            retained: 0,
            symbol: HashMap::new(),
            code: HashMap::new(),
            change: Vec::new(),
            altered: Default::default(),
            occupied: false,
            removal: Vec::new(),
            insertion: Vec::new(),
        };
        index.frame.resize(index.state.frame.len(), Vec::new());
        for world in 0..index.state.world.len() {
            index.insert(world);
        }
        index
    }

    fn insert(&mut self, world: usize) {
        let site = self.vacant.pop().unwrap_or_else(|| {
            let site = self.rank.len();
            self.rank.push(0);
            site
        });
        self.rank[site] = world;
        self.site.push(site);
        self.insertion.push(site);
        let value = &self.state.world[world];
        self.occupied = true;
        self.altered
            .extend(value.particle.iter().map(|token| token.value));
        self.frame[value.frame].push(site);
        self.retained += 1;
        for token in &value.particle {
            let count = self.symbol.entry(token.value).or_default();
            if *count == 0 {
                self.change.push((token.value, true));
            }
            *count += 1;
            if let Symbol::Rule(rule) = token.value {
                self.code
                    .entry(rule)
                    .or_default()
                    .push((site, token.id, token.capture.unwrap()));
                self.retained += 1;
            }
            let key = (value.frame, Term::new(token.value, token.capture));
            let posting = self.term.entry(key).or_default();
            if !posting.contains(&site) {
                posting.push(site);
                self.retained += 1;
            }
        }
    }

    pub(crate) fn advance(&mut self, state: Arc<State>, removed: &Set<usize>) {
        self.altered.clear();
        self.removal.clear();
        self.insertion.clear();
        self.occupied = false;
        for &world in removed {
            let site = self.site[world];
            self.removal.push(site);
            let value = &self.state.world[world];
            self.occupied = true;
            self.altered
                .extend(value.particle.iter().map(|token| token.value));
            self.frame[value.frame].retain(|&candidate| candidate != site);
            self.retained -= 1;
            for token in &value.particle {
                let count = self.symbol.get_mut(&token.value).unwrap();
                *count -= 1;
                if *count == 0 {
                    self.symbol.remove(&token.value);
                    self.change.push((token.value, false));
                }
                if let Symbol::Rule(rule) = token.value {
                    self.retained -= 1;
                    let code = self.code.get_mut(&rule).unwrap();
                    code.retain(|&(candidate, id, _)| candidate != site || id != token.id);
                    if code.is_empty() {
                        self.code.remove(&rule);
                    }
                }
                let key = (value.frame, Term::new(token.value, token.capture));
                let Some(posting) = self.term.get_mut(&key) else {
                    continue;
                };
                let previous = posting.len();
                posting.retain(|&candidate| candidate != site);
                self.retained -= previous - posting.len();
                if posting.is_empty() {
                    self.term.remove(&key);
                }
            }
            self.vacant.push(site);
        }
        self.site = self
            .site
            .iter()
            .enumerate()
            .filter_map(|(world, &site)| (!removed.contains(&world)).then_some(site))
            .collect();
        for (world, &site) in self.site.iter().enumerate() {
            self.rank[site] = world;
        }
        self.state = state;
        self.frame.resize(self.state.frame.len(), Vec::new());
        for world in self.site.len()..self.state.world.len() {
            self.insert(world);
        }
    }

    pub fn candidate(&self, pattern: &[Term], frame: usize) -> Vec<usize> {
        let Some(world) = self.frame.get(frame) else {
            return Vec::new();
        };
        let posting = pattern
            .iter()
            .map(|term| {
                let key = (frame, Term::new(term.value, term.capture));
                self.term.get(&key)
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
        let mut result = candidate
            .iter()
            .filter(|site| posting.iter().all(|posting| posting.contains(site)))
            .map(|&site| self.rank[site])
            .collect::<Vec<_>>();
        result.sort_unstable();
        result
    }

    pub(crate) fn site(&self, world: usize) -> usize {
        self.site[world]
    }

    pub(crate) fn world(&self, site: usize) -> usize {
        self.rank[site]
    }

    pub fn retained(&self) -> usize {
        self.frame.len()
            + self.term.len()
            + self.retained
            + self.site.len()
            + self.rank.len()
            + self.vacant.len()
            + self.symbol.len()
            + self.code.len()
            + self.change.len()
            + self.altered.len()
            + self.removal.len()
            + self.insertion.len()
    }

    pub(crate) fn change(&mut self) -> impl Iterator<Item = (Symbol, bool)> + '_ {
        self.change.drain(..)
    }

    pub(crate) fn frame(&self) -> impl Iterator<Item = usize> + '_ {
        self.frame
            .iter()
            .enumerate()
            .filter_map(|(index, world)| (index == 0 || !world.is_empty()).then_some(index))
    }

    pub(crate) fn code(&self, rule: usize) -> impl Iterator<Item = (usize, usize, usize)> + '_ {
        self.code
            .get(&rule)
            .into_iter()
            .flatten()
            .map(|&(site, token, capture)| (self.rank[site], token, capture))
    }

    pub(crate) fn available(&self) -> impl Iterator<Item = Symbol> + '_ {
        self.symbol.keys().copied()
    }

    pub(crate) fn contains(&self, symbol: &Symbol) -> bool {
        self.symbol.contains_key(symbol)
    }
}
