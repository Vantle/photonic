use super::Search;
use crate::ordering::Ordering;
use crate::program::Symbol;
use crate::refinement::Refinement;
use crate::state::{Canonical, Frame, State, Token, World};
use std::sync::Arc;

fn reference(state: &State) -> (Canonical, usize) {
    let refinement = Refinement::new(state);
    let world = Ordering::new(0..state.world.len(), |index| {
        let world = &state.world[index];
        let mut particle = world
            .particle
            .iter()
            .map(|token| token.value)
            .collect::<Vec<_>>();
        particle.sort();
        (state.chain(world.frame), particle, refinement.world[index])
    });
    let world = if state.world.len() >= 4 {
        world.quotient(|| crate::symmetry::world(state))
    } else {
        world
    };
    let mut best: Option<Canonical> = None;
    let mut work = 1;
    for world in world {
        work += 1;
        let mut occupied = vec![Vec::new(); state.frame.len()];
        let mut capture = vec![Vec::new(); state.frame.len()];
        for (position, &source) in world.iter().enumerate() {
            let value = &state.world[source];
            occupied[value.frame].push(position);
            for token in &value.particle {
                if let Some(frame) = token.capture {
                    capture[frame].push((position, token.value));
                }
            }
        }
        for capture in &mut capture {
            capture.sort_unstable();
        }
        for frame in Ordering::new(
            state.reachable().into_iter().filter(|&index| index != 0),
            |index| {
                (
                    state.chain(index),
                    &occupied[index],
                    &capture[index],
                    refinement.frame[index],
                )
            },
        ) {
            work += 1;
            let order = std::iter::once(0).chain(frame).collect::<Vec<_>>();
            let value = state.rename(&world, &order);
            if best.as_ref().is_none_or(|best| value.state < best.state) {
                best = Some(value);
            }
        }
    }
    (best.unwrap(), work)
}

fn verify(state: State) {
    let incidence = crate::incidence::Incidence::new(&state);
    let frame = state.reachable();
    for world in [
        (0..state.world.len()).collect::<Vec<_>>(),
        (0..state.world.len()).rev().collect(),
    ] {
        for frame in [
            frame.clone(),
            std::iter::once(0)
                .chain(frame[1..].iter().rev().copied())
                .collect(),
        ] {
            let expected = state.rename(&world, &frame);
            let actual = super::renaming::rename(&state, &incidence, &world, &frame);
            assert_eq!(actual.state, expected.state);
            assert_eq!(actual.world, expected.world);
            assert_eq!(actual.frame, expected.frame);
            assert_eq!(actual.resource, expected.resource);
        }
    }
    let (expected, work) = reference(&state);
    let mut search = Search::new(Arc::new(state));
    for position in 1..=work {
        assert_eq!(search.step(), position == work);
    }
    assert!(search.step());
    let actual = search.finish().unwrap();
    assert_eq!(actual.state, expected.state);
    assert_eq!(actual.world, expected.world);
    assert_eq!(actual.frame, expected.frame);
    assert_eq!(actual.resource, expected.resource);
}

#[test]
fn membership() {
    let particle = vec![
        Token {
            id: usize::MAX,
            value: Symbol::Rule(0),
            capture: Some(2),
        },
        Token {
            id: 7,
            value: Symbol::Atom(0),
            capture: None,
        },
        Token {
            id: 101,
            value: Symbol::Atom(0),
            capture: None,
        },
    ];
    let state = State {
        world: vec![
            Arc::new(World {
                frame: 1,
                particle: particle.iter().rev().cloned().collect(),
            }),
            Arc::new(World {
                frame: 2,
                particle: particle.iter().chain(&particle).cloned().collect(),
            }),
        ]
        .into(),
        frame: (0..4)
            .map(|index| {
                Arc::new(Frame {
                    scope: index,
                    parent: (index > 0).then_some(0),
                    lexical: (index > 0).then_some(index.saturating_sub(1)),
                    particle: particle.iter().cloned().collect(),
                    held: particle.clone(),
                })
            })
            .collect(),
    };
    assert_eq!(state.reachable(), vec![0, 1, 2]);
    verify(state);
}

#[test]
fn demand() {
    let program = crate::program::Program::new(&frontend::lowering::parse("A,B, [A] C").unwrap());
    let state = State::initial(&program);
    let mut search = Search::new(Arc::new(state.clone()));
    assert!(search.refinement.get().is_none());
    while !search.step() {
        assert!(search.refinement.get().is_none());
    }
    assert!(search.refinement.get().is_none());
    verify(state);
    let program = crate::program::Program::new(&frontend::lowering::parse("A,A").unwrap());
    let state = State::initial(&program);
    let search = Search::new(Arc::new(state.clone()));
    assert!(search.refinement.get().is_some());
    verify(state);
    let state = State {
        world: Default::default(),
        frame: (0..3)
            .map(|index| {
                Arc::new(Frame {
                    scope: 0,
                    parent: (index > 0).then_some(0),
                    lexical: (index > 0).then_some(0),
                    held: Vec::new(),
                    particle: if index == 0 {
                        (1..3)
                            .map(|capture| Token {
                                id: capture,
                                value: Symbol::Rule(capture),
                                capture: Some(capture),
                            })
                            .collect()
                    } else {
                        Default::default()
                    },
                })
            })
            .collect(),
    };
    let mut search = Search::new(Arc::new(state.clone()));
    assert!(search.refinement.get().is_none());
    assert!(!search.step());
    assert!(search.refinement.get().is_some());
    verify(state);
}

#[test]
fn differential() {
    let mut seed = 7u64;
    for iteration in 0..256 {
        let count = 1 + iteration % 3;
        let mut select = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            seed >> 61 == 0
        };
        let token = (0..8)
            .map(|index| Token {
                id: 17 + index * 11,
                value: if index % 3 == 0 {
                    Symbol::Rule(index % 2)
                } else {
                    Symbol::Atom(index % 3)
                },
                capture: (index % 3 == 0).then_some((index + iteration) % count),
            })
            .collect::<Vec<_>>();
        let frame = (0..count)
            .map(|index| {
                Arc::new(Frame {
                    scope: (index + iteration) % 2,
                    parent: (index > 0).then_some(0),
                    lexical: (index > 0).then_some(0),
                    particle: token.iter().filter(|_| select()).cloned().collect(),
                    held: token.iter().filter(|_| select()).cloned().collect(),
                })
            })
            .collect();
        let world = (0..iteration % 5)
            .map(|index| {
                Arc::new(World {
                    frame: index % count,
                    particle: token.iter().filter(|_| select()).cloned().collect(),
                })
            })
            .collect();
        let state = State { world, frame };
        verify(state.clone());
        verify(State {
            world: state.world.iter().rev().cloned().collect(),
            ..state
        });
    }
}

struct Random(u64);

impl Random {
    fn below(&mut self, bound: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) % bound as u64) as usize
    }

    fn shuffle<Value>(&mut self, value: &mut [Value]) {
        for index in (1..value.len()).rev() {
            value.swap(index, self.below(index + 1));
        }
    }
}

fn generate(random: &mut Random) -> State {
    let count = 1 + random.below(4);
    let resource = (0..1 + random.below(12))
        .map(|position| {
            let rule = random.below(3) == 0;
            Token {
                id: 1000 + position * 7,
                value: if rule {
                    Symbol::Rule(random.below(2))
                } else {
                    Symbol::Atom(random.below(3))
                },
                capture: rule.then(|| random.below(count)),
            }
        })
        .collect::<Vec<_>>();
    let mut world = (0..random.below(9))
        .map(|_| (random.below(count), Vec::new()))
        .collect::<Vec<_>>();
    let mut particle = vec![Vec::new(); count];
    let mut held = vec![Vec::new(); count];
    for token in &resource {
        for _ in 0..1 + usize::from(random.below(3) == 0) {
            match random.below(3) {
                0 if !world.is_empty() => {
                    let target = random.below(world.len());
                    world[target].1.push(token.clone());
                }
                1 if token.capture.is_some() => particle[random.below(count)].push(token.clone()),
                _ => held[random.below(count)].push(token.clone()),
            }
        }
    }
    State {
        world: world
            .into_iter()
            .map(|(frame, particle)| Arc::new(World { frame, particle }))
            .collect(),
        frame: (0..count)
            .map(|index| {
                let mut particle = std::mem::take(&mut particle[index]);
                particle.sort_by_key(|token| token.id);
                particle.dedup_by_key(|token| token.id);
                Arc::new(Frame {
                    scope: random.below(2),
                    parent: (index > 0).then(|| random.below(index)),
                    lexical: (index > 0).then(|| random.below(index)),
                    particle: particle.into_iter().collect(),
                    held: std::mem::take(&mut held[index]),
                })
            })
            .collect(),
    }
}

fn renumber(state: &State, random: &mut Random) -> State {
    let count = state.frame.len();
    let mut order = (1..count).collect::<Vec<_>>();
    random.shuffle(&mut order);
    let mapping = std::iter::once(0)
        .chain((1..count).map(|index| 1 + order.iter().position(|&frame| frame == index).unwrap()))
        .collect::<Vec<_>>();
    let offset = 5000 + random.below(1000);
    let stride = 1 + 2 * random.below(5);
    let token = |token: &Token| Token {
        id: offset + (1_000_000 - token.id) * stride,
        value: token.value,
        capture: token.capture.map(|frame| mapping[frame]),
    };
    let mut world = state
        .world
        .iter()
        .map(|world| {
            Arc::new(World {
                frame: mapping[world.frame],
                particle: world.particle.iter().rev().map(token).collect(),
            })
        })
        .collect::<Vec<_>>();
    random.shuffle(&mut world);
    let mut frame = vec![None; count];
    for (index, value) in state.frame.iter().enumerate() {
        let mut particle = value.particle.iter().map(token).collect::<Vec<_>>();
        particle.sort_by_key(|token| token.id);
        frame[mapping[index]] = Some(Arc::new(Frame {
            scope: value.scope,
            parent: value.parent.map(|frame| mapping[frame]),
            lexical: value.lexical.map(|frame| mapping[frame]),
            particle: particle.into_iter().collect(),
            held: value.held.iter().rev().map(token).collect(),
        }));
    }
    State {
        world: world.into_iter().collect(),
        frame: frame.into_iter().map(Option::unwrap).collect(),
    }
}

#[test]
fn renumbering() {
    let mut random = Random(7);
    for _ in 0..4096 {
        let state = generate(&mut random);
        let expected = state.canonical().state;
        assert_eq!(expected.canonical().state, expected, "{state:?}");
        for _ in 0..4 {
            let renamed = renumber(&state, &mut random);
            assert_eq!(
                renamed.canonical().state,
                expected,
                "{state:?}\n{renamed:?}"
            );
        }
        let mut world = state.world.iter().cloned().collect::<Vec<_>>();
        let Some(first) = world.first_mut() else {
            continue;
        };
        Arc::make_mut(first).particle.push(Token {
            id: 999_999,
            value: Symbol::Atom(3),
            capture: None,
        });
        let changed = State {
            world: world.into_iter().collect(),
            frame: state.frame.clone(),
        };
        assert_ne!(changed.canonical().state, expected, "{state:?}");
    }
}
