use crate::canonical::Exhausted;
use crate::link::Link;
use crate::opening::Opening;
use crate::profile;
use crate::program::{Program, Scope, Symbol};
use crate::snapshot::Form;
use hashing::Builder;
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

// A configuration in its canonical form, and where its coherences, frames and token ids went.
#[derive(Clone)]
pub struct Canonical {
    pub state: State,
    pub renaming: Renaming,
}

// Where a configuration's coherences, frames and token ids go in another: none for a coherence or
// frame it drops.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Renaming {
    pub world: Vec<Option<usize>>,
    pub frame: Vec<Option<usize>>,
    pub resource: crate::relation::Map<usize, usize>,
}

impl Renaming {
    pub(crate) fn place(&self, place: crate::place::Place) -> Option<crate::place::Place> {
        use crate::place::Place;
        Some(match place {
            Place::World(index, id) => Place::World(self.world[index]?, self.resource[&id]),
            Place::Context(index, id) => Place::Context(self.frame[index]?, self.resource[&id]),
            Place::Held(index, id) => Place::Held(self.frame[index]?, self.resource[&id]),
        })
    }

    // A flow into the renamed configuration, which holds this many coherences and frames.
    pub(crate) fn flow(
        &self,
        flow: crate::flow::Flow,
        world: usize,
        frame: usize,
    ) -> crate::flow::Flow {
        let resource = flow
            .resource
            .into_iter()
            .filter_map(|(place, basis)| Some((self.place(place)?, basis)))
            .collect();
        let mut context = vec![crate::basis::Set::default(); world];
        for (index, target) in self.world.iter().enumerate() {
            if let Some(target) = target {
                context[*target] = flow.context[index].clone();
            }
        }
        let mut mapping = vec![None; frame];
        for (index, target) in self.frame.iter().enumerate() {
            if let Some(target) = target {
                mapping[*target] = flow.frame[index];
            }
        }
        crate::flow::Flow {
            resource,
            context,
            frame: mapping,
        }
    }
}

impl State {
    pub(crate) fn token(&self, place: crate::place::Place) -> Option<&Token> {
        use crate::place::Place;
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

    #[inline]
    pub(crate) fn ancestry(&self, frame: usize) -> impl Iterator<Item = usize> + '_ {
        std::iter::successors(Some(frame), |&frame| self.frame[frame].lexical)
    }

    pub(crate) fn visible(
        &self,
        frame: usize,
    ) -> impl Iterator<Item = (crate::place::Place, &Token)> {
        self.ancestry(frame).flat_map(|frame| {
            self.frame[frame]
                .particle
                .iter()
                .map(move |token| (crate::place::Place::Context(frame, token.id), token))
        })
    }

    pub(crate) fn resolve(
        &self,
        location: crate::location::Location,
        id: usize,
    ) -> Option<crate::place::Place> {
        if let Some(world) = location.world() {
            let place = crate::place::Place::World(world, id);
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
        Self::load(&program.scope, &program.scope[0])
    }

    pub(crate) fn target(program: &Program, source: &frontend::source::Program) -> Self {
        let (program, root) = program.target(source);
        Self::load(&program.scope, &root)
    }

    fn load(scope: &[Scope], root: &Scope) -> Self {
        let mut state = Self {
            world: crate::sequence::List::default(),
            frame: crate::sequence::List::default(),
        };
        let mut opening = Opening {
            scope,
            held: &[],
            base: &[],
            capture: 0,
            vacant: std::iter::empty(),
            next: 0,
            opened: Vec::new(),
        };
        opening.open(&mut state, 0, root, None, None);
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
        let frame = (0..state.frame.len()).collect::<Vec<_>>();
        state.rename(&world, &frame).state
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

    // A search with a single candidate ordering stays open for two steps and finishes on its
    // third, which the unit of work its caller spends on the configuration pays for; every later
    // step spends one unit of the budget, and the search gives up once the budget is spent.
    pub(crate) fn canonical(&self, budget: &mut usize) -> Result<Canonical, Exhausted> {
        let _scope = profile::Scope::new(profile::Phase::Normalization);
        let mut search = crate::canonical::Search::new(Arc::new(self.clone()));
        let mut open = 0;
        while !search.step() {
            open += 1;
            if open > 2 {
                *budget = budget.checked_sub(1).ok_or(Exhausted)?;
            }
        }
        Ok(search
            .finish()
            .expect("a finished search names its configuration"))
    }

    // The configuration renamed without a search: its coherences and reachable frames in the order
    // it holds them, and its tokens numbered as a canonical form numbers them.
    pub(crate) fn listed(&self) -> Canonical {
        let world = (0..self.world.len()).collect::<Vec<_>>();
        self.rename(&world, &self.reachable())
    }

    // The configuration as a report shows it: in its canonical form when that takes at most the
    // budget, else listed.
    pub(crate) fn show(&self, mut budget: usize) -> (Canonical, Form) {
        match self.canonical(&mut budget) {
            Ok(named) => (named, Form::Canonical),
            Err(Exhausted) => (self.listed(), Form::Listed),
        }
    }

    pub(crate) fn rename(&self, world: &[usize], frame: &[usize]) -> Canonical {
        let _scope = profile::Scope::new(profile::Phase::Renaming);
        self.remap(world, frame, |mapping| {
            let count = world
                .iter()
                .map(|&index| self.world[index].particle.len())
                .chain(frame.iter().map(|&index| self.frame[index].size()))
                .sum();
            let mut incidence = HashMap::<
                usize,
                (Symbol, Option<usize>, SmallVec<[(Link, usize); 1]>),
                Builder,
            >::with_capacity_and_hasher(count, Builder);
            let mut link = |token: &Token, kind: Link, position: usize| {
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
                    .push((kind, position));
            };
            for (position, &index) in world.iter().enumerate() {
                for token in &self.world[index].particle {
                    link(token, Link::Particle, position);
                }
            }
            for (position, &index) in frame.iter().enumerate() {
                for token in &self.frame[index].held {
                    link(token, Link::Holder, position);
                }
            }
            for (position, &index) in frame.iter().enumerate() {
                for token in &self.frame[index].particle {
                    link(token, Link::Owner, position);
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
            renaming: Renaming {
                world: rename,
                frame: mapping,
                resource,
            },
        }
    }

    // The frames a rule owned by a captured frame sees, as one empty coherence in that frame.
    pub(crate) fn environment(
        &self,
        capture: usize,
        budget: &mut usize,
    ) -> Result<Canonical, Exhausted> {
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
        .canonical(budget)
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
