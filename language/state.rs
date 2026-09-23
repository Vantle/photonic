use crate::hashing::Builder;
use crate::link::Link;
use crate::program::{Program, Symbol};
use smallvec::SmallVec;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Token {
    pub id: usize,
    pub value: Symbol,
    pub capture: Option<usize>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct World {
    pub frame: usize,
    pub particle: Vec<Token>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Frame {
    pub scope: usize,
    pub parent: Option<usize>,
    pub lexical: Option<usize>,
    pub particle: crate::population::Set,
    pub held: Vec<Token>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct State {
    pub world: crate::sequence::List<Arc<World>>,
    pub frame: crate::sequence::List<Arc<Frame>>,
}

pub struct Canonical {
    pub state: State,
    pub world: Vec<Option<usize>>,
    pub frame: Vec<Option<usize>>,
    pub resource: crate::relation::Map<usize, usize>,
}

impl State {
    pub(crate) fn token(&self, place: crate::flow::Place) -> Option<&Token> {
        use crate::flow::Place;
        match place {
            Place::World(index, id) => self
                .world
                .get(index)?
                .particle
                .iter()
                .find(|token| token.id == id),
            Place::Context(index, id) => self
                .frame
                .get(index)?
                .particle
                .iter()
                .find(|token| token.id == id),
            Place::Held(index, id) => self
                .frame
                .get(index)?
                .held
                .iter()
                .find(|token| token.id == id),
        }
    }

    pub(crate) fn visible(
        &self,
        frame: usize,
    ) -> impl Iterator<Item = (crate::flow::Place, &Token)> {
        std::iter::successors(Some(frame), |&frame| self.frame[frame].lexical).flat_map(|frame| {
            self.frame[frame]
                .particle
                .iter()
                .map(move |token| (crate::flow::Place::Context(frame, token.id), token))
        })
    }

    pub(crate) fn resolve(
        &self,
        location: crate::location::Location,
        id: usize,
    ) -> Option<crate::flow::Place> {
        if let Some(world) = location.world() {
            let place = crate::flow::Place::World(world, id);
            if self.token(place).is_some() {
                return Some(place);
            }
        }
        self.visible(location.frame(self))
            .find_map(|(place, token)| (token.id == id).then_some(place))
    }

    pub(crate) fn reclaim(mut self, reachable: &[usize]) -> Self {
        self.frame.truncate(reachable.last().unwrap() + 1);
        if self.frame.len() == reachable.len() {
            return self;
        }
        for index in 0..self.frame.len() {
            let frame = &self.frame[index];
            if reachable.binary_search(&index).is_ok()
                || (frame.scope == 0
                    && frame.parent.is_none()
                    && frame.lexical.is_none()
                    && frame.held.is_empty()
                    && frame.particle.is_empty())
            {
                continue;
            }
            let frame = Arc::make_mut(&mut self.frame[index]);
            frame.scope = 0;
            frame.parent = None;
            frame.lexical = None;
            frame.held.clear();
            frame.particle.clear();
        }
        self
    }

    pub fn initial(program: &Program) -> Self {
        Self::configuration(&program.initial, &program.scope[0].rule)
    }

    pub(crate) fn configuration(initial: &[Vec<Symbol>], rule: &[usize]) -> Self {
        let mut id = 0;
        let state = Self {
            world: initial
                .iter()
                .map(|particle| {
                    World {
                        frame: 0,
                        particle: particle
                            .iter()
                            .map(|&value| Token::new(value, 0, &mut id))
                            .collect(),
                    }
                    .into()
                })
                .collect(),
            frame: vec![
                Frame {
                    scope: 0,
                    parent: None,
                    lexical: None,
                    particle: rule
                        .iter()
                        .map(|&rule| Token::new(Symbol::Rule(rule), 0, &mut id))
                        .collect(),
                    held: Vec::new(),
                }
                .into(),
            ]
            .into(),
        };
        let mut world = (0..state.world.len()).collect::<Vec<_>>();
        world.sort_by_key(|&index| {
            let mut particle = state.world[index]
                .particle
                .iter()
                .map(|token| token.value)
                .collect::<Vec<_>>();
            particle.sort();
            particle
        });
        state.rename(&world, &[0]).state
    }

    pub fn reachable(&self) -> Vec<usize> {
        self.closure(
            std::iter::once(0).chain(self.world.iter().flat_map(|world| world.reference())),
        )
    }

    pub(crate) fn closure(&self, root: impl IntoIterator<Item = usize>) -> Vec<usize> {
        let mut selected = SmallVec::<[bool; 64]>::from_elem(false, self.frame.len());
        let mut pending = SmallVec::<[usize; 16]>::new();
        for index in root {
            if !std::mem::replace(&mut selected[index], true) {
                pending.push(index);
            }
        }
        while let Some(index) = pending.pop() {
            for target in self.frame[index].reference() {
                if !std::mem::replace(&mut selected[target], true) {
                    pending.push(target);
                }
            }
        }
        selected
            .into_iter()
            .enumerate()
            .filter_map(|(index, selected)| selected.then_some(index))
            .collect()
    }

    pub(crate) fn chain(&self, frame: usize) -> Vec<(usize, Vec<Symbol>)> {
        let mut chain = Vec::new();
        let mut cursor = Some(frame);
        while let Some(index) = cursor {
            let frame = &self.frame[index];
            let mut held = frame
                .held
                .iter()
                .map(|token| token.value)
                .collect::<Vec<_>>();
            held.sort();
            chain.push((frame.scope, held));
            cursor = frame.parent;
        }
        chain.reverse();
        chain
    }

    pub fn canonical(&self) -> Canonical {
        #[cfg(feature = "measurement")]
        let _measurement = crate::measurement::profile::Scope::new(
            crate::measurement::profile::Phase::Normalization,
        );
        let mut search = crate::canonical::Search::new(std::sync::Arc::new(self.clone()));
        while !search.step() {}
        search
            .finish()
            .expect("a finite configuration has a canonical ordering")
    }

    pub(crate) fn rename(&self, world: &[usize], frame: &[usize]) -> Canonical {
        #[cfg(feature = "measurement")]
        let _measurement =
            crate::measurement::profile::Scope::new(crate::measurement::profile::Phase::Renaming);
        self.remap(world, frame, |mapping| {
            let mut incidence = HashMap::<
                usize,
                (Symbol, Option<usize>, SmallVec<[(Link, usize); 1]>),
                Builder,
            >::default();
            for (position, &index) in world.iter().enumerate() {
                for token in &self.world[index].particle {
                    incidence
                        .entry(token.id)
                        .or_insert_with(|| {
                            (
                                token.value,
                                token.capture.and_then(|index| mapping[index]),
                                SmallVec::new(),
                            )
                        })
                        .2
                        .push((Link::Particle, position));
                }
            }
            for (position, &index) in frame.iter().enumerate() {
                for token in &self.frame[index].held {
                    incidence
                        .entry(token.id)
                        .or_insert_with(|| {
                            (
                                token.value,
                                token.capture.and_then(|index| mapping[index]),
                                SmallVec::new(),
                            )
                        })
                        .2
                        .push((Link::Holder, position));
                }
            }
            for (position, &index) in frame.iter().enumerate() {
                for token in &self.frame[index].particle {
                    incidence
                        .entry(token.id)
                        .or_insert_with(|| {
                            (
                                token.value,
                                token.capture.and_then(|index| mapping[index]),
                                SmallVec::new(),
                            )
                        })
                        .2
                        .push((Link::Owner, position));
                }
            }
            let mut resource = incidence.into_iter().collect::<Vec<_>>();
            resource.sort_by(|left, right| left.1.cmp(&right.1).then_with(|| left.0.cmp(&right.0)));
            resource
                .iter()
                .enumerate()
                .map(|(position, (index, _))| (*index, position))
                .collect::<crate::relation::Map<_, _>>()
        })
    }

    pub(crate) fn remap(
        &self,
        world: &[usize],
        frame: &[usize],
        resource: impl FnOnce(&[Option<usize>]) -> crate::relation::Map<usize, usize>,
    ) -> Canonical {
        let mut mapping = vec![None; self.frame.len()];
        for (position, &index) in frame.iter().enumerate() {
            mapping[index] = Some(position);
        }
        let resource = resource(&mapping);
        let token = |token: &Token| Token {
            id: resource[&token.id],
            value: token.value,
            capture: token.capture.and_then(|index| mapping[index]),
        };
        let particle = |mut value: Vec<Token>| {
            value.sort_by_key(|token| token.id);
            value
        };
        let state = Self {
            world: world
                .iter()
                .map(|&index| {
                    World {
                        frame: mapping[self.world[index].frame].unwrap(),
                        particle: particle(self.world[index].particle.iter().map(token).collect()),
                    }
                    .into()
                })
                .collect(),
            frame: frame
                .iter()
                .map(|&index| {
                    let value = &self.frame[index];
                    Frame {
                        scope: value.scope,
                        parent: value.parent.and_then(|index| mapping[index]),
                        lexical: value.lexical.and_then(|index| mapping[index]),
                        particle: particle(value.particle.iter().map(token).collect()).into(),
                        held: particle(value.held.iter().map(token).collect()),
                    }
                    .into()
                })
                .collect(),
        };
        let mut rename = vec![None; self.world.len()];
        for (position, &index) in world.iter().enumerate() {
            rename[index] = Some(position);
        }
        Canonical {
            state,
            world: rename,
            frame: mapping,
            resource,
        }
    }

    pub fn size(&self) -> usize {
        self.world
            .iter()
            .map(|world| world.particle.len())
            .sum::<usize>()
            + self
                .reachable()
                .into_iter()
                .map(|index| self.frame[index].size())
                .sum::<usize>()
    }

    pub fn environment(&self, capture: usize) -> Self {
        Self {
            world: vec![
                World {
                    frame: capture,
                    particle: Vec::new(),
                }
                .into(),
            ]
            .into(),
            frame: self.frame.clone(),
        }
        .canonical()
        .state
    }
}

impl World {
    pub(crate) fn reference(&self) -> impl Iterator<Item = usize> + '_ {
        std::iter::once(self.frame).chain(self.particle.iter().filter_map(|token| token.capture))
    }
}

impl Frame {
    pub(crate) fn token(&self) -> impl Iterator<Item = &Token> {
        self.particle.iter().chain(&self.held)
    }

    pub(crate) fn reference(&self) -> impl Iterator<Item = usize> + '_ {
        let capture = self.particle.capture();
        let scanned = capture
            .is_none()
            .then(|| self.particle.iter())
            .into_iter()
            .flatten();
        self.parent
            .into_iter()
            .chain(self.lexical)
            .chain(capture.filter(|_| !self.particle.is_empty()))
            .chain(scanned.chain(&self.held).filter_map(|token| token.capture))
    }

    pub(crate) fn size(&self) -> usize {
        self.particle.len() + self.held.len()
    }
}

#[cfg(test)]
#[path = "test/occurrence.rs"]
mod test;

impl Token {
    pub(crate) fn new(value: Symbol, capture: usize, next: &mut usize) -> Self {
        let token = Self {
            id: *next,
            value,
            capture: matches!(value, Symbol::Rule(_)).then_some(capture),
        };
        *next += 1;
        token
    }
}
