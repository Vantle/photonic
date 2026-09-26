use crate::flow::Binding;
use crate::place::Place;
use crate::program::Program;
use crate::runtime::{Limit, Runtime};
use crate::state::State;
use std::collections::BTreeSet;
use std::sync::Arc;

fn binding(state: &State, value: &Binding) -> (Vec<Place>, Vec<Place>, Vec<Place>) {
    let canonical = state.canonical();
    let map = |value: &crate::basis::Set<Place>| {
        let mut value = value
            .iter()
            .map(|place| match *place {
                Place::World(world, token) => {
                    Place::World(canonical.world[world].unwrap(), canonical.resource[&token])
                }
                Place::Context(frame, token) => {
                    Place::Context(canonical.frame[frame].unwrap(), canonical.resource[&token])
                }
                Place::Held(frame, token) => {
                    Place::Held(canonical.frame[frame].unwrap(), canonical.resource[&token])
                }
            })
            .collect::<Vec<_>>();
        value.sort_unstable();
        value
    };
    (map(&value.footprint), map(&value.exact), map(&value.read))
}

#[test]
fn reference() {
    for source in [
        "A, [A] B, [A] C",
        "A.A.A, [A.A] B",
        "A.X,B.X, [A,B] (C, D)",
        "A, [A] (B, C), [B,C] D",
        "A,A,B, [A,B] C",
        "A.X,A.Y,B, [A,B] (C, D)",
        "Seed.A, [Seed] ().([A] B)",
        "A, [A] (B, [B] C)",
        "A.X, [A] ((B, [B] C), D), [C,D] E",
        "A.([A] B),A.([A] C)",
        "A,B, [A] (C, [C,B] D)",
        "A, [A] (), [ ] B",
        "A,A,A, [A,A] B",
        "Seed.A.X, [Seed] (([A] B), ([A] C))",
    ] {
        let program = Arc::new(Program::new(&frontend::lowering::parse(source).unwrap()));
        let initial = Arc::new(State::initial(&program));
        let mut graph = Runtime::seed(program.clone(), initial);
        graph.run(100_000, Limit::default());
        for state in graph.state.iter().take(32) {
            let mut runtime = Runtime::seed(program.clone(), state.clone());
            runtime.run(100_000, Limit::default());
            let snapshot = runtime.snapshot();
            let expected = snapshot
                .event
                .iter()
                .filter(|event| event.source == 0 && event.evidence.contains(&0))
                .map(|event| {
                    (
                        runtime.state[event.target].canonical().state,
                        event.rule,
                        (
                            event.footprint.clone(),
                            event.exact.clone(),
                            event.read.clone(),
                        ),
                    )
                })
                .collect::<BTreeSet<_>>();
            let mut search = crate::reduction::Search::new(program.clone(), state.clone());
            let mut actual = BTreeSet::new();
            for _ in 0..100_000 {
                let work = search.work;
                if let Some(event) = search.run(Limit::default()) {
                    assert_eq!(
                        event.fingerprint.value,
                        crate::fingerprint::state(&event.state)
                    );
                    let layout = crate::layout::Layout::new(&event.state);
                    assert_eq!(event.fingerprint.layout.cell, layout.cell);
                    assert_eq!(event.fingerprint.layout.resource, layout.resource);
                    assert_eq!(event.fingerprint.layout.reach.frame, layout.reach.frame);
                    actual.insert((
                        event.state.canonical().state,
                        event.rule,
                        binding(state, &event.binding),
                    ));
                }
                if search.work == work {
                    break;
                }
            }
            assert_eq!(actual, expected, "{source} {state:?}");
        }
    }
}

#[test]
fn fingerprint() {
    for source in [
        "A.X,B.X, [A,B] C",
        "A, [A] (B, [B] C)",
        "Seed.A, [Seed] ().([A] B)",
    ] {
        let mut runtime = Runtime::new(&frontend::lowering::parse(source).unwrap());
        runtime.run(100_000, Limit::default());
        for state in &runtime.state {
            let expected = crate::fingerprint::state(state);
            let world = (0..state.world.len()).rev().collect::<Vec<_>>();
            let frame = std::iter::once(0)
                .chain((1..state.frame.len()).rev())
                .collect::<Vec<_>>();
            let renamed = state.rename(&world, &frame).state;
            assert_eq!(expected, crate::fingerprint::state(&renamed));
        }
    }
}

#[test]
fn incremental() {
    for source in [
        "A, [A] (B, [B] C), [C] A",
        "Seed.A.X, [Seed] (([A] B), ([A] C))",
        "A.X,B.Y, [A,B] ((C, [C] D), E), [D,E] F",
        "A, [A] (B, C), [B] D, [C] E, [D,E] F",
        "A.([A] B), [B] (C, [C] A)",
        "A.X,B.Y,Z, [Z] Q, [Q] R, [A,B] End",
        "A.X,B.X,C.X, [A,B] D, [C] E, [D,E] F",
        "A, [A] (B, [B] C), [C] (D, [D] A)",
        "A.([A] B),A.([A] C), [B,C] D",
        "A,A,A, [A,A] B, [B,A] C",
        "A,A,B, [A,B] C, [A,C] D",
        "A, [A] B, [B] A, [ ] Z",
    ] {
        let program = Arc::new(Program::new(&frontend::lowering::parse(source).unwrap()));
        let mut state = Arc::new(State::initial(&program));
        let mut cached = crate::reduction::Search::new(program.clone(), state.clone());
        for _ in 0..64 {
            let mut chosen = None;
            let mut actual = BTreeSet::new();
            for _ in 0..10000 {
                let work = cached.work;
                if let Some(event) = cached.run(Limit::default()) {
                    actual.insert((
                        event.rule,
                        event.state.canonical().state,
                        binding(&state, &event.binding),
                    ));
                    if chosen.is_none() {
                        chosen = Some(event);
                    }
                }
                if cached.work == work {
                    break;
                }
            }
            let mut fresh = crate::reduction::Search::new(program.clone(), state.clone());
            let mut expected = BTreeSet::new();
            for _ in 0..10000 {
                let work = fresh.work;
                if let Some(event) = fresh.run(Limit::default()) {
                    expected.insert((
                        event.rule,
                        event.state.canonical().state,
                        binding(&state, &event.binding),
                    ));
                }
                if fresh.work == work {
                    break;
                }
            }
            assert_eq!(actual, expected, "{source}");
            let Some(event) = chosen else {
                assert!(expected.is_empty(), "{source}");
                break;
            };
            assert!(
                expected.contains(&(
                    event.rule,
                    event.state.canonical().state,
                    binding(&state, &event.binding)
                )),
                "{source}"
            );
            assert_eq!(
                event.fingerprint.value,
                crate::fingerprint::state(&event.state),
                "{source}"
            );
            let layout = crate::layout::Layout::new(&event.state);
            assert_eq!(event.fingerprint.layout.cell, layout.cell);
            assert_eq!(event.fingerprint.layout.resource, layout.resource);
            assert_eq!(event.fingerprint.layout.reach.frame, layout.reach.frame);
            state = event.state.clone();
            cached.advance(event.state, &event.change, event.fingerprint);
        }
    }
}

#[test]
fn scaling() {
    for width in [128, 8192] {
        let mut source = String::from("A,Stage.0,\n");
        for index in 0..width {
            source.push_str(&format!("[A,A] Never.{index},\n"));
        }
        for index in 0..128 {
            source.push_str(&format!("[Stage.{index}] Stage.{},\n", index + 1));
        }
        let program = Arc::new(Program::new(&frontend::lowering::parse(&source).unwrap()));
        let state = Arc::new(State::initial(&program));
        let mut search = crate::reduction::Search::new(program, state);
        let mut count = 0;
        for _ in 0..10_000 {
            let work = search.work;
            if let Some(event) = search.run(Limit {
                occurrence: width + 131,
                ..Limit::default()
            }) {
                count += 1;
                search.advance(event.state, &event.change, event.fingerprint);
            }
            if search.work == work {
                break;
            }
        }
        assert_eq!(count, 128);
        assert_eq!(search.preparation(), 129);
    }
}
