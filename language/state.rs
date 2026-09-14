use crate::program::{Program, Symbol};
use std::collections::{BTreeSet, HashMap};

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
    pub held: Vec<Token>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct State {
    pub world: Vec<World>,
    pub frame: Vec<Frame>,
}

pub struct Canonical {
    pub state: State,
    pub world: Vec<Option<usize>>,
    pub frame: Vec<Option<usize>>,
    pub resource: HashMap<usize, usize>,
}

impl State {
    pub fn initial(program: &Program) -> Self {
        let mut id = 0;
        let state = Self {
            world: program
                .initial
                .iter()
                .map(|particle| World {
                    frame: 0,
                    particle: particle
                        .iter()
                        .map(|&value| {
                            let token = Token {
                                id,
                                value,
                                capture: matches!(value, Symbol::Rule(_)).then_some(0),
                            };
                            id += 1;
                            token
                        })
                        .collect(),
                })
                .collect(),
            frame: vec![Frame {
                scope: 0,
                parent: None,
                lexical: None,
                held: Vec::new(),
            }],
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

    pub fn reachable(&self) -> BTreeSet<usize> {
        let mut selected = vec![false; self.frame.len()];
        let mut pending = Vec::new();
        let mut insert = |index| {
            if !std::mem::replace(&mut selected[index], true) {
                pending.push(index);
            }
        };
        insert(0);
        for world in &self.world {
            insert(world.frame);
            for capture in world.particle.iter().filter_map(|token| token.capture) {
                insert(capture);
            }
        }
        while let Some(index) = pending.pop() {
            let frame = &self.frame[index];
            for target in frame
                .parent
                .into_iter()
                .chain(frame.lexical)
                .chain(frame.held.iter().filter_map(|token| token.capture))
            {
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
        let mut search = crate::canonical::Search::new(std::sync::Arc::new(self.clone()));
        while !search.step() {}
        search
            .finish()
            .expect("a finite configuration has a canonical ordering")
    }

    pub(crate) fn rename(&self, world: &[usize], frame: &[usize]) -> Canonical {
        let mut mapping = vec![None; self.frame.len()];
        for (position, &index) in frame.iter().enumerate() {
            mapping[index] = Some(position);
        }
        let mut incidence = HashMap::<usize, (Symbol, Option<usize>, Vec<(bool, usize)>)>::new();
        for (position, &index) in world.iter().enumerate() {
            for token in &self.world[index].particle {
                incidence
                    .entry(token.id)
                    .or_insert_with(|| {
                        (
                            token.value,
                            token.capture.and_then(|index| mapping[index]),
                            Vec::new(),
                        )
                    })
                    .2
                    .push((false, position));
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
                            Vec::new(),
                        )
                    })
                    .2
                    .push((true, position));
            }
        }
        let mut resource = incidence.into_iter().collect::<Vec<_>>();
        resource.sort_by(|left, right| left.1.cmp(&right.1).then_with(|| left.0.cmp(&right.0)));
        let resource = resource
            .into_iter()
            .enumerate()
            .map(|(position, (index, _))| (index, position))
            .collect::<HashMap<_, _>>();
        let particle = |value: &[Token]| {
            let mut value = value
                .iter()
                .map(|token| Token {
                    id: resource[&token.id],
                    value: token.value,
                    capture: token.capture.and_then(|index| mapping[index]),
                })
                .collect::<Vec<_>>();
            value.sort_by_key(|token| token.id);
            value
        };
        let state = Self {
            world: world
                .iter()
                .map(|&index| World {
                    frame: mapping[self.world[index].frame].unwrap(),
                    particle: particle(&self.world[index].particle),
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
                        held: particle(&value.held),
                    }
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
                .map(|index| self.frame[index].held.len())
                .sum::<usize>()
    }

    pub fn environment(&self, capture: usize) -> State {
        Self {
            world: vec![World {
                frame: capture,
                particle: Vec::new(),
            }],
            frame: self.frame.clone(),
        }
        .canonical()
        .state
    }
}
