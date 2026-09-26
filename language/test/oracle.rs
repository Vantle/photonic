use crate::location::Location;
use crate::place::Place;
use crate::program::{Program, Symbol};
use crate::state::{Frame, State, Token, World};
use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};
use std::sync::Arc;
use std::task::Poll;

#[derive(Debug, Eq, Hash, PartialEq)]
struct Transition {
    rule: usize,
    read: Place,
    world: BTreeSet<usize>,
    resource: BTreeSet<Place>,
    state: State,
}

#[derive(Clone, Default)]
struct Selection {
    location: Vec<Location>,
    resource: BTreeSet<Place>,
}

fn reachable(state: &State) -> BTreeSet<usize> {
    let mut result = BTreeSet::from([0]);
    for world in &state.world {
        result.insert(world.frame);
        result.extend(world.particle.iter().filter_map(|token| token.capture));
    }
    loop {
        let previous = result.clone();
        for &frame in &previous {
            let value = &state.frame[frame];
            result.extend(value.parent);
            result.extend(value.lexical);
            result.extend(
                value
                    .particle
                    .iter()
                    .chain(&value.held)
                    .filter_map(|token| token.capture),
            );
        }
        if result == previous {
            return result;
        }
    }
}

fn visible(state: &State, location: Location) -> Vec<(Place, &Token)> {
    let mut result = Vec::new();
    if let Location::World(world) = location {
        result.extend(
            state.world[world]
                .particle
                .iter()
                .map(|token| (Place::World(world, token.id), token)),
        );
    }
    let mut cursor = Some(location.frame(state));
    while let Some(frame) = cursor {
        result.extend(
            state.frame[frame]
                .particle
                .iter()
                .map(|token| (Place::Context(frame, token.id), token)),
        );
        cursor = state.frame[frame].lexical;
    }
    result
}

fn particle(
    pattern: &[Symbol],
    available: &[(Place, &Token)],
    owner: usize,
    selected: &BTreeSet<Place>,
) -> Vec<BTreeSet<Place>> {
    let Some((&symbol, remaining)) = pattern.split_first() else {
        return vec![selected.clone()];
    };
    let mut result = Vec::new();
    for &(place, token) in available {
        if token.value != symbol
            || (matches!(symbol, Symbol::Rule(_)) && token.capture != Some(owner))
            || selected.contains(&place)
        {
            continue;
        }
        let mut selected = selected.clone();
        selected.insert(place);
        result.extend(particle(remaining, available, owner, &selected));
    }
    result
}

fn selection(state: &State, pattern: &[Vec<Symbol>], frame: usize, owner: usize) -> Vec<Selection> {
    let location = state
        .world
        .iter()
        .enumerate()
        .filter(|(_, world)| world.frame == frame)
        .map(|(world, _)| Location::World(world))
        .chain([Location::Context(frame)])
        .collect::<Vec<_>>();
    let mut result = vec![Selection::default()];
    for pattern in pattern {
        let mut next = Vec::new();
        for selected in result {
            for &location in &location {
                if selected.location.contains(&location)
                    || (pattern.is_empty() && matches!(location, Location::Context(_)))
                {
                    continue;
                }
                for resource in particle(
                    pattern,
                    &visible(state, location),
                    owner,
                    &selected.resource,
                ) {
                    let mut value = selected.clone();
                    value.location.push(location);
                    value.resource = resource;
                    next.push(value);
                }
            }
        }
        result = next;
    }
    result
}

fn introduce(value: Symbol, capture: usize, next: &mut usize) -> Token {
    let token = Token {
        id: *next,
        value,
        capture: matches!(value, Symbol::Rule(_)).then_some(capture),
    };
    *next += 1;
    token
}

fn apply(
    program: &Program,
    state: &State,
    frame: usize,
    owner: usize,
    rule: usize,
    selected: &Selection,
) -> State {
    let consumed = selected
        .location
        .iter()
        .filter_map(|location| location.world())
        .collect::<BTreeSet<_>>();
    let mut next = state
        .world
        .iter()
        .flat_map(|world| &world.particle)
        .chain(
            state
                .frame
                .iter()
                .flat_map(|frame| frame.particle.iter().chain(&frame.held)),
        )
        .map(|token| token.id + 1)
        .max()
        .unwrap_or(0);
    let mut result = State {
        world: state
            .world
            .iter()
            .enumerate()
            .filter(|(world, _)| !consumed.contains(world))
            .map(|(_, value)| value.clone())
            .collect(),
        frame: state
            .frame
            .iter()
            .enumerate()
            .map(|(frame, value)| {
                let mut value = (**value).clone();
                value
                    .particle
                    .retain(|token| !selected.resource.contains(&Place::Context(frame, token.id)));
                Arc::new(value)
            })
            .collect(),
    };
    let mut remainder = BTreeMap::new();
    for &world in &consumed {
        for token in &state.world[world].particle {
            if !selected.resource.contains(&Place::World(world, token.id)) {
                remainder.insert(token.id, token.clone());
            }
        }
    }
    let returning = frame == owner && frame != 0;
    let parent = if returning {
        state.frame[frame].parent.unwrap()
    } else {
        frame
    };
    let mut held = BTreeMap::new();
    for &location in &selected.location {
        for (place, token) in visible(state, location) {
            if selected.resource.contains(&place) {
                held.insert(token.id, token.clone());
            }
        }
    }
    if returning {
        for token in &state.frame[frame].held {
            held.insert(token.id, token.clone());
        }
    }
    for output in &program.rule[rule].output {
        let destination = if let Some(scope) = output.body {
            let destination = result.frame.len();
            result.frame.push(Arc::new(Frame {
                scope,
                parent: Some(parent),
                lexical: Some(owner),
                particle: program.scope[scope]
                    .rule
                    .iter()
                    .map(|&rule| introduce(Symbol::Rule(rule), destination, &mut next))
                    .collect(),
                held: held.values().cloned().collect(),
            }));
            destination
        } else {
            parent
        };
        let particle = remainder
            .values()
            .cloned()
            .chain(
                output
                    .particle
                    .iter()
                    .map(|&value| introduce(value, owner, &mut next)),
            )
            .collect();
        result.world.push(Arc::new(World {
            frame: destination,
            particle,
        }));
    }
    result.canonical().state
}

fn expected(program: &Program, state: &State) -> HashSet<Transition> {
    let mut result = HashSet::new();
    for frame in reachable(state) {
        let mut available = visible(state, Location::Context(frame));
        for (world, value) in state
            .world
            .iter()
            .enumerate()
            .filter(|(_, world)| world.frame == frame)
        {
            available.extend(
                value
                    .particle
                    .iter()
                    .map(|token| (Place::World(world, token.id), token)),
            );
        }
        for (read, token) in available {
            let Symbol::Rule(rule) = token.value else {
                continue;
            };
            let owner = token.capture.unwrap();
            if program.rule[rule].input.is_empty()
                && matches!(read, Place::Context(context, _) if context != frame)
            {
                continue;
            }
            for selected in selection(state, &program.rule[rule].input, frame, owner) {
                if let Place::World(world, _) = read
                    && !program.rule[rule].input.is_empty()
                    && !selected.location.contains(&Location::World(world))
                {
                    continue;
                }
                result.insert(Transition {
                    rule,
                    read,
                    world: selected
                        .location
                        .iter()
                        .filter_map(|location| location.world())
                        .collect(),
                    state: apply(program, state, frame, owner, rule, &selected),
                    resource: selected.resource,
                });
            }
        }
    }
    result
}

fn actual(program: &Arc<Program>, state: &Arc<State>) -> HashSet<Transition> {
    let mut search = crate::reduction::Search::new(program.clone(), state.clone());
    let limit = crate::runtime::Limit {
        configuration: 4096,
        record: 1_000_000,
        coherence: 64,
        occurrence: 4096,
        scope: 64,
    };
    let mut result = HashSet::new();
    for _ in 0..100_000 {
        let event = match search.run(limit) {
            Poll::Ready(Some(event)) => event,
            Poll::Ready(None) => return result,
            Poll::Pending => continue,
        };
        result.insert(Transition {
            rule: event.rule,
            read: *event.binding.read.first().unwrap(),
            world: event.binding.world.iter().copied().collect(),
            resource: event.binding.exact.iter().copied().collect(),
            state: event.state.canonical().state,
        });
    }
    panic!("reference comparison did not complete");
}

#[test]
fn occurrence() {
    for source in [
        "[] A",
        "[()] A",
        "(),(), [(), ()] A",
        "A, [A] B, [[A] B] C",
        "A,A, [A] B, [A] B, [([A] B).([A] B)] C",
        "([A] B).A,X",
        "A, [A] ([B] C), [[B] C] D",
        "A.X, [A] (B, C), [B,C] D",
        "A.X, [A] (Seed, [Seed] B), [B] C",
        "A, [A] (B, [B] C, [[B] C] D)",
        "A, [A] (B, [B] (C, [C] D))",
        "A, [A] (B), [B] C, [[A] (B)] D",
        "A, [A] (B, [B] C), [B] D",
        "A, [A] (B, [] C, [B,C] D)",
    ] {
        verify(source, 3);
    }
}

fn verify(source: &str, maximum: usize) {
    let program = Arc::new(Program::new(&frontend::lowering::parse(source).unwrap()));
    let initial = State::initial(&program);
    let mut seen = HashSet::from([initial.clone()]);
    let mut pending = VecDeque::from([(initial, 0)]);
    while let Some((state, depth)) = pending.pop_front() {
        assert_eq!(
            state.reachable().into_iter().collect::<BTreeSet<_>>(),
            reachable(&state),
            "{source}, depth {depth}"
        );
        let state = Arc::new(state);
        let expected = expected(&program, &state);
        assert_eq!(
            actual(&program, &state),
            expected,
            "{source}, depth {depth}"
        );
        if depth == maximum {
            continue;
        }
        for transition in expected {
            if seen.insert(transition.state.clone()) {
                pending.push_back((transition.state, depth + 1));
            }
        }
    }
}

#[test]
fn generated() {
    for data in ["", "()", "A", "A.A", "A,B", "A.A,B", "A.([A] B)"] {
        for input in ["", "()", "A", "A.A", "A,B", "[A] B", "A.([A] B)"] {
            for output in ["B", "(B, C)", "([A] B)", "(A, [A] B)"] {
                verify(&format!("[A] B, [{input}] {output}, {data}"), 2);
            }
        }
    }
}

#[test]
fn incremental() {
    for source in [
        "A.X, [A] (B, [B] C), [C] (D, [D] E), [E] F",
        "A, [A] (B, [] C, [B,C] D), [D] E",
        "A, [A] B, [A] B, [[A] B] C, [C] (D, [D] E)",
        "A.X, [A] (B, C), [B,C] D, [D] (E, [E] F)",
    ] {
        let program = Arc::new(Program::new(&frontend::lowering::parse(source).unwrap()));
        let mut state = Arc::new(State::initial(&program));
        let mut search = crate::reduction::Search::new(program.clone(), state.clone());
        let limit = crate::runtime::Limit {
            configuration: 4096,
            record: 1_000_000,
            coherence: 64,
            occurrence: 4096,
            scope: 64,
        };
        for step in 0..8 {
            let expected = expected(&program, &state);
            let mut result = HashSet::new();
            let mut selected = None;
            for _ in 0..100_000 {
                let event = match search.run(limit) {
                    Poll::Ready(Some(event)) => event,
                    Poll::Ready(None) => break,
                    Poll::Pending => continue,
                };
                result.insert(Transition {
                    rule: event.rule,
                    read: *event.binding.read.first().unwrap(),
                    world: event.binding.world.iter().copied().collect(),
                    resource: event.binding.exact.iter().copied().collect(),
                    state: event.state.canonical().state,
                });
                selected = Some(event);
            }
            assert_eq!(result, expected, "{source}, step {step}");
            let Some(event) = selected else {
                break;
            };
            if step % 2 == 0 {
                search.evict();
            }
            state = event.state;
            search.advance(
                state.clone(),
                &event.change,
                event.fingerprint,
                event.layout,
            );
        }
    }
}

#[test]
fn depth() {
    for depth in [1, 2, 3, 8, 16, 32] {
        let mut source = format!("Stage{depth}, [Stage{depth}] End");
        for level in (0..depth).rev() {
            source = format!("Stage{level}, [Stage{level}] ({source})");
        }
        verify(&source, depth + 1);
    }
}
