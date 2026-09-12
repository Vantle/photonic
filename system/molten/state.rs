use crate::program::{Program, Symbol};
use std::collections::{BTreeMap, BTreeSet, HashMap};

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

fn permutation(value: &mut [usize], position: usize, visit: &mut impl FnMut(&[usize])) {
    if position == value.len() {
        visit(value);
        return;
    }
    for index in position..value.len() {
        value.swap(position, index);
        permutation(value, position + 1, visit);
        value.swap(position, index);
    }
}

fn arrangement(
    group: &mut [Vec<usize>],
    prefix: &mut Vec<usize>,
    visit: &mut impl FnMut(&[usize]),
) {
    let Some((head, tail)) = group.split_first_mut() else {
        visit(prefix);
        return;
    };
    permutation(head, 0, &mut |choice| {
        let length = prefix.len();
        prefix.extend_from_slice(choice);
        arrangement(tail, prefix, visit);
        prefix.truncate(length);
    });
}

fn ordering<Key: Ord>(
    value: impl IntoIterator<Item = usize>,
    key: impl Fn(usize) -> Key,
    mut visit: impl FnMut(&[usize]),
) {
    let mut group = BTreeMap::<Key, Vec<usize>>::new();
    for index in value {
        group.entry(key(index)).or_default().push(index);
    }
    arrangement(
        &mut group.into_values().collect::<Vec<_>>(),
        &mut Vec::new(),
        &mut visit,
    );
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
        state.canonical().state
    }

    pub fn reachable(&self) -> BTreeSet<usize> {
        let mut selected = BTreeSet::new();
        let mut pending = vec![0];
        for world in &self.world {
            pending.push(world.frame);
            pending.extend(world.particle.iter().filter_map(|token| token.capture));
        }
        while let Some(index) = pending.pop() {
            if !selected.insert(index) {
                continue;
            }
            let frame = &self.frame[index];
            pending.extend(frame.parent);
            pending.extend(frame.lexical);
            pending.extend(frame.held.iter().filter_map(|token| token.capture));
        }
        selected
    }

    fn chain(&self, frame: usize) -> Vec<(usize, Vec<Symbol>)> {
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
        let retained = self.reachable();
        let mut best: Option<Canonical> = None;
        ordering(
            0..self.world.len(),
            |index| {
                let world = &self.world[index];
                let mut particle = world
                    .particle
                    .iter()
                    .map(|token| token.value)
                    .collect::<Vec<_>>();
                particle.sort();
                (self.chain(world.frame), particle)
            },
            |world| {
                ordering(
                    retained.iter().copied().filter(|&index| index != 0),
                    |index| {
                        let occupied = world
                            .iter()
                            .enumerate()
                            .filter_map(|(position, &source)| {
                                (self.world[source].frame == index).then_some(position)
                            })
                            .collect::<Vec<_>>();
                        let mut capture = world
                            .iter()
                            .enumerate()
                            .flat_map(|(position, &source)| {
                                self.world[source]
                                    .particle
                                    .iter()
                                    .filter(move |token| token.capture == Some(index))
                                    .map(move |token| (position, token.value))
                            })
                            .collect::<Vec<_>>();
                        capture.sort();
                        (self.chain(index), occupied, capture)
                    },
                    |arrangement| {
                        let mut frame = vec![0];
                        frame.extend_from_slice(arrangement);
                        let mut mapping = vec![None; self.frame.len()];
                        for (position, &index) in frame.iter().enumerate() {
                            mapping[index] = Some(position);
                        }
                        let mut incidence =
                            HashMap::<usize, (Symbol, Option<usize>, Vec<(bool, usize)>)>::new();
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
                        resource.sort_by(|left, right| {
                            left.1.cmp(&right.1).then_with(|| left.0.cmp(&right.0))
                        });
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
                        if best.as_ref().is_some_and(|best| best.state <= state) {
                            return;
                        }
                        let mut rename = vec![None; self.world.len()];
                        for (position, &index) in world.iter().enumerate() {
                            rename[index] = Some(position);
                        }
                        best = Some(Canonical {
                            state,
                            world: rename,
                            frame: mapping,
                            resource,
                        });
                    },
                );
            },
        );
        best.expect("a finite configuration has a canonical ordering")
    }

    pub fn cells(&self) -> usize {
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
