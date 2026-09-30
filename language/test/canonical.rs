use super::division::Division;
use super::{LIMIT, Search, Stage};
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
        (state.chain(world.frame), particle, refinement.world(index))
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
                    refinement.frame(index),
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

// Whether the search names the configuration by its parts: its whole search is too long and the
// anchors leave two or more parts.
fn divided(state: &State, work: usize) -> bool {
    work > LIMIT && Division::new(Arc::new(state.clone()), &Refinement::new(state)).is_some()
}

fn refined(search: &Search) -> bool {
    match &search.stage {
        Stage::Whole(whole) => whole.refined(),
        Stage::Division(_) | Stage::Complete(_) => panic!("the search is no longer whole"),
    }
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
            assert_eq!(actual.renaming, expected.renaming);
        }
    }
    let (expected, work) = reference(&state);
    let divided = divided(&state, work);
    let mut search = Search::new(Arc::new(state.clone()));
    if divided {
        while !search.step() {}
        let actual = search.finish().unwrap();
        assert_eq!(crate::test::canonical(&actual.state).state, actual.state);
        assert_eq!(actual.state.world.len(), state.world.len());
        assert_eq!(actual.state.frame.len(), state.reachable().len());
        return;
    }
    for position in 1..=work {
        assert_eq!(search.step(), position == work);
    }
    assert!(search.step());
    let actual = search.finish().unwrap();
    assert_eq!(actual.state, expected.state);
    assert_eq!(actual.renaming, expected.renaming);
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
    assert!(!refined(&search));
    while !search.step() {
        if let Stage::Whole(_) = &search.stage {
            assert!(!refined(&search));
        }
    }
    verify(state);
    let program = crate::program::Program::new(&frontend::lowering::parse("A,A").unwrap());
    let state = State::initial(&program);
    let search = Search::new(Arc::new(state.clone()));
    assert!(refined(&search));
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
    assert!(!refined(&search));
    assert!(!search.step());
    assert!(refined(&search));
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

// A frame being built, with the tokens it owns and holds.
#[derive(Clone)]
struct Draft {
    scope: usize,
    parent: Option<usize>,
    lexical: Option<usize>,
    particle: Vec<Token>,
    held: Vec<Token>,
}

// A configuration being built from copies of random parts.
struct Assembly {
    world: Vec<World>,
    frame: Vec<Draft>,
    next: usize,
}

impl Assembly {
    fn new() -> Self {
        Self {
            world: Vec::new(),
            frame: vec![Draft {
                scope: 0,
                parent: None,
                lexical: None,
                particle: Vec::new(),
                held: Vec::new(),
            }],
            next: 1,
        }
    }

    fn token(&mut self, value: Symbol, capture: Option<usize>) -> Token {
        self.next += 3;
        Token {
            id: self.next,
            value,
            capture,
        }
    }

    fn frame(&mut self, scope: usize, parent: usize, lexical: usize) -> usize {
        self.frame.push(Draft {
            scope,
            parent: Some(parent),
            lexical: Some(lexical),
            particle: Vec::new(),
            held: Vec::new(),
        });
        self.frame.len() - 1
    }

    // One copy of a part drawn from its own seed, so every copy of a seed is the same part up to
    // its ids: scopes under the parent, coherences in them or in the parent, tokens shared inside
    // the part, rules capturing its frames, the root or the parent, tokens its frames hold, and
    // tokens it shares with every other copy.
    fn part(&mut self, seed: u64, parent: usize, common: &[Token]) {
        let mut random = Random(seed);
        let mut frame = vec![parent];
        for _ in 0..random.below(3) {
            let above = frame[random.below(frame.len())];
            let lexical = frame[random.below(frame.len())];
            frame.push(self.frame(random.below(3), above, lexical));
        }
        let token = (0..1 + random.below(4))
            .map(|_| {
                if random.below(3) == 0 {
                    let capture = [0, parent, frame[random.below(frame.len())]][random.below(3)];
                    self.token(Symbol::Rule(random.below(2)), Some(capture))
                } else {
                    self.token(Symbol::Atom(random.below(3)), None)
                }
            })
            .collect::<Vec<_>>();
        let count = random.below(3) + usize::from(frame.len() == 1);
        for index in 0..count.max(frame.len() - 1) {
            let place = if index + 1 < frame.len() {
                frame[index + 1]
            } else {
                frame[random.below(frame.len())]
            };
            let mut particle = Vec::new();
            for _ in 0..1 + random.below(3) {
                particle.push(token[random.below(token.len())].clone());
            }
            if !common.is_empty() && random.below(2) == 0 {
                particle.push(common[random.below(common.len())].clone());
            }
            self.world.push(World {
                frame: place,
                particle,
            });
        }
        for &index in &frame[1..] {
            if random.below(2) == 0 {
                let rule = self.token(Symbol::Rule(random.below(2)), Some(index));
                self.frame[index].particle.push(rule);
            }
            for _ in 0..random.below(3) {
                let held = token[random.below(token.len())].clone();
                self.frame[index].held.push(held);
            }
            if !common.is_empty() && random.below(2) == 0 {
                let held = common[random.below(common.len())].clone();
                self.frame[index].held.push(held);
            }
        }
        if random.below(4) == 0 {
            let held = token[random.below(token.len())].clone();
            self.frame[0].held.push(held);
        }
    }

    fn finish(self) -> State {
        State {
            world: self.world.into_iter().map(Arc::new).collect(),
            frame: self
                .frame
                .into_iter()
                .map(|draft| {
                    let mut particle = draft.particle;
                    particle.sort_by_key(|token| token.id);
                    particle.dedup_by_key(|token| token.id);
                    Arc::new(Frame {
                        scope: draft.scope,
                        parent: draft.parent,
                        lexical: draft.lexical,
                        particle: particle.into_iter().collect(),
                        held: draft.held,
                    })
                })
                .collect(),
        }
    }
}

// Copies of random parts: at the root, or under a scope whose frames all copies hold a token of,
// nested a level deeper at times, with a few distinct parts beside them.
fn replicate(random: &mut Random, copy: usize) -> State {
    let mut assembly = Assembly::new();
    let mut parent = 0;
    let mut common = Vec::new();
    for _ in 0..random.below(3) {
        let hub = assembly.frame(random.below(3), parent, parent);
        let particle = vec![assembly.token(Symbol::Atom(random.below(3)), None)];
        assembly.world.push(World {
            frame: hub,
            particle,
        });
        if random.below(2) == 0 {
            let shared = assembly.token(Symbol::Atom(random.below(3)), None);
            assembly.frame[hub].held.push(shared.clone());
            common.push(shared);
        }
        parent = hub;
    }
    for _ in 0..1 + random.below(2) {
        let seed = random.below(1 << 30) as u64;
        for _ in 0..copy {
            assembly.part(seed, parent, &common);
        }
    }
    for _ in 0..random.below(3) {
        let seed = random.below(1 << 30) as u64;
        assembly.part(seed, random.below(assembly.frame.len()), &common);
    }
    assembly.finish()
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

// The configuration with one more token in its first coherence, which no renaming undoes.
fn perturb(state: &State) -> Option<State> {
    let mut world = state.world.iter().cloned().collect::<Vec<_>>();
    Arc::make_mut(world.first_mut()?).particle.push(Token {
        id: 999_999,
        value: Symbol::Atom(3),
        capture: None,
    });
    Some(State {
        world: world.into_iter().collect(),
        frame: state.frame.clone(),
    })
}

#[test]
fn renumbering() {
    let mut random = Random(7);
    for _ in 0..4096 {
        let state = generate(&mut random);
        let expected = crate::test::canonical(&state).state;
        assert_eq!(
            crate::test::canonical(&expected).state,
            expected,
            "{state:?}"
        );
        for _ in 0..4 {
            let renamed = renumber(&state, &mut random);
            assert_eq!(
                crate::test::canonical(&renamed).state,
                expected,
                "{state:?}\n{renamed:?}"
            );
        }
        let Some(changed) = perturb(&state) else {
            continue;
        };
        assert_ne!(
            crate::test::canonical(&changed).state,
            expected,
            "{state:?}"
        );
    }
}

// The steps a search takes to name a configuration.
fn steps(state: &State) -> usize {
    let mut search = Search::new(Arc::new(state.clone()));
    (1..).find(|_| search.step()).unwrap()
}

#[test]
fn division() {
    let mut random = Random(11);
    let mut split = 0;
    for iteration in 0..1536 {
        let state = replicate(&mut random, 2 + iteration % 7);
        let named = crate::test::canonical(&state);
        let expected = named.state;
        assert_eq!(expected.world.len(), state.world.len());
        assert_eq!(expected.frame.len(), state.reachable().len());
        assert_eq!(
            crate::test::canonical(&expected).state,
            expected,
            "{state:?}"
        );
        for _ in 0..3 {
            let renamed = renumber(&state, &mut random);
            assert_eq!(
                crate::test::canonical(&renamed).state,
                expected,
                "{state:?}\n{renamed:?}"
            );
        }
        if let Some(changed) = perturb(&state) {
            assert_ne!(
                crate::test::canonical(&changed).state,
                expected,
                "{state:?}"
            );
        }
        let mut search = Search::new(Arc::new(state.clone()));
        search.step();
        split += usize::from(matches!(search.stage, Stage::Division(_)));
    }
    assert!(
        split > 256,
        "{split} configurations were named by their parts"
    );
}

// Every ordering of the coherences and of the frames other than the root, with the least renamed
// configuration: equal for two configurations exactly when one is a renaming of the other.
fn brute(state: &State) -> State {
    let mut world = (0..state.world.len()).collect::<Vec<_>>();
    let frame = state.reachable();
    let mut best: Option<State> = None;
    loop {
        let mut order = frame[1..].to_vec();
        loop {
            let frame = std::iter::once(0)
                .chain(order.iter().copied())
                .collect::<Vec<_>>();
            let value = state.rename(&world, &frame).state;
            if best.as_ref().is_none_or(|best| value < *best) {
                best = Some(value);
            }
            if !permute(&mut order) {
                break;
            }
        }
        if !permute(&mut world) {
            break;
        }
    }
    best.unwrap()
}

// The next ordering in lexicographic order, or false after the last, which it leaves sorted.
fn permute(value: &mut [usize]) -> bool {
    let Some(pivot) = (1..value.len())
        .rev()
        .find(|&index| value[index - 1] < value[index])
    else {
        value.reverse();
        return false;
    };
    let next = (pivot..value.len())
        .rev()
        .find(|&index| value[pivot - 1] < value[index])
        .unwrap();
    value.swap(pivot - 1, next);
    value[pivot..].reverse();
    true
}

// A division's name, however small the configuration's whole search.
fn divide(state: &State) -> Option<State> {
    let mut division = Division::new(Arc::new(state.clone()), &Refinement::new(state))?;
    loop {
        if let Some(named) = division.step() {
            return Some(named.state);
        }
    }
}

#[test]
fn isomorphism() {
    let mut random = Random(13);
    let mut checked = 0;
    for iteration in 0..4096 {
        let state = replicate(&mut random, 2 + iteration % 2);
        if state.world.len() > 5 || state.reachable().len() > 4 {
            continue;
        }
        let Some(named) = divide(&state) else {
            continue;
        };
        checked += 1;
        assert_eq!(brute(&named), brute(&state), "{state:?}");
        for _ in 0..3 {
            let renamed = renumber(&state, &mut random);
            assert_eq!(
                divide(&renamed),
                Some(named.clone()),
                "{state:?}\n{renamed:?}"
            );
        }
    }
    assert!(checked > 256, "{checked} small configurations divided");
}

fn scope(count: usize, body: &str) -> String {
    vec![body; count].join(", ")
}

#[test]
fn program() {
    for (source, bound) in [
        (format!("{}, [Q] R", scope(12, "(K, [Q] R)")), 64),
        (
            format!("Go, [Go] (M, [Q] R, {})", scope(12, "(K, [Q] R)")),
            64,
        ),
        (
            format!("Go, [Go] (M, [Q] R, ({}))", scope(9, "(K, [Q] R)")),
            64,
        ),
        (scope(6, &format!("(M, {})", scope(6, "(K, [Q] R)"))), 128),
    ] {
        let program = crate::program::Program::new(&frontend::lowering::parse(&source).unwrap());
        let state = State::initial(&program);
        assert!(steps(&state) <= bound, "{source}: {} steps", steps(&state));
        let mut random = Random(17);
        let expected = crate::test::canonical(&state).state;
        for _ in 0..4 {
            let renamed = renumber(&state, &mut random);
            assert_eq!(crate::test::canonical(&renamed).state, expected, "{source}");
        }
    }
}

#[test]
fn exploration() {
    for (source, bound) in [
        ("[] (K, [Q] R)".to_owned(), 256),
        (
            format!("{}, [Seed] (A, A)", scope(6, "Seed.X")),
            256,
        ),
        (
            format!("Go, [Go] (M, [Q] R, {})", scope(10, "(K, [Q] R)")),
            256,
        ),
        ("Seed.X, [Seed] ((A, [Q] R), (A, [Q] R), (A, [Q] R), (A, [Q] R), (A, [Q] R), (A, [Q] R))".to_owned(), 256),
    ] {
        let program = frontend::lowering::parse(&source).unwrap();
        let mut runtime = crate::runtime::Runtime::new(&program);
        runtime.run(
            200_000,
            crate::runtime::Limit {
                scope: 16,
                ..Default::default()
            },
        );
        assert!(runtime.work < 200_000, "{source}");
        for state in &runtime.state {
            assert!(steps(state) <= bound, "{source}: {} steps", steps(state));
        }
    }
}

#[test]
fn budget() {
    let ring = State {
        world: (0..12)
            .map(|index| {
                Arc::new(World {
                    frame: 0,
                    particle: vec![
                        Token {
                            id: index,
                            value: Symbol::Atom(0),
                            capture: None,
                        },
                        Token {
                            id: (index + 1) % 12,
                            value: Symbol::Atom(0),
                            capture: None,
                        },
                    ],
                })
            })
            .collect(),
        frame: vec![Arc::new(Frame {
            scope: 0,
            parent: None,
            lexical: None,
            particle: Default::default(),
            held: Vec::new(),
        })]
        .into(),
    };
    let mut budget = 1000;
    assert!(ring.canonical(&mut budget).is_err());
    assert_eq!(budget, 0);
    let program = crate::program::Program::new(&frontend::lowering::parse("A, B").unwrap());
    let state = State::initial(&program);
    let mut budget = 0;
    assert!(state.canonical(&mut budget).is_ok());
    let program = crate::program::Program::new(&frontend::lowering::parse("A, A, B.C").unwrap());
    let state = State::initial(&program);
    let taken = steps(&state);
    let mut budget = taken - 3;
    assert_eq!(
        state.canonical(&mut budget).unwrap().state,
        crate::test::canonical(&state).state
    );
    assert_eq!(budget, 0);
    let mut budget = taken - 4;
    assert!(state.canonical(&mut budget).is_err());
}
