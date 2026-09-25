use super::support::{configuration, flat, vocabulary};
use crate::emit;
use crate::execution;
use crate::lift;
use crate::vocabulary::Vocabulary;
use code::configuration::Configuration;
use code::program::Program;
use machine::exploration::{explore, successor};
use machine::flat::Flat;
use machine::limit::Limit;
use machine::state::State;
use photonic::prism::Outcome;
use random::Generator;
use std::collections::HashSet;

fn bound() -> photonic::runtime::Limit {
    photonic::runtime::Limit {
        state: 10_000,
        record: 10_000_000,
        world: 16,
        cell: 64,
        frame: 10,
    }
}

fn limit() -> Limit {
    Limit {
        state: 64,
        ..Limit::default()
    }
}

fn start(configuration: &Configuration) -> State {
    State::new(configuration).expect("differential inputs are flat")
}

fn reachable(program: &Flat, initial: State) -> Vec<State> {
    let mut seen = HashSet::from([initial.key(256).unwrap()]);
    let mut queue = vec![initial];
    let mut result = Vec::new();
    while let Some(state) = queue.pop() {
        for (next, identity) in successor(program, &state, &limit()).unwrap_or_default() {
            if seen.insert(identity) {
                queue.push(next);
            }
        }
        result.push(state);
    }
    result
}

fn exhaustive(program: &Program, initial: &Configuration, known: &Vocabulary) -> usize {
    let compiled = Flat::new(program).unwrap();
    let source = emit::program(program, initial, known);
    let mut search = photonic::prism::Search::new(source.clone(), source);
    search.run(5_000_000, Some(bound()));
    let mut checked = 0;
    for state in reachable(&compiled, start(initial)) {
        if !state.membership().is_empty() {
            continue;
        }
        search.target(emit::program(program, &state.configuration(), known));
        assert_eq!(
            search.verdict().outcome,
            Outcome::Reached,
            "{}\n{}\nmissing {}",
            crate::text::configuration(initial, known),
            crate::text::program(program, known),
            crate::text::configuration(&state.configuration(), known),
        );
        checked += 1;
    }
    checked
}

fn direct(program: &Program, initial: &Configuration, known: &Vocabulary) -> bool {
    let exploration = explore(
        &Flat::new(program).unwrap(),
        start(initial),
        &limit(),
        |_| false,
    );
    assert!(exploration.complete());
    if exploration
        .terminal
        .iter()
        .any(|state| !state.membership().is_empty())
    {
        return false;
    }
    let reached = exploration
        .terminal
        .iter()
        .filter(|terminal| {
            let mut search = photonic::path::Search::new(
                emit::program(program, initial, known),
                emit::program(program, &terminal.configuration(), known),
            );
            search.run(1_000_000, bound());
            search.summary().outcome == Outcome::Reached
        })
        .count();
    assert!(
        reached >= 1,
        "{}\n{}",
        crate::text::configuration(initial, known),
        crate::text::program(program, known)
    );
    if exploration.terminal.len() == 1 {
        assert_eq!(reached, 1);
    }
    true
}

fn kernel(program: &Program, initial: &Configuration, known: &Vocabulary) {
    let machine = explore(
        &Flat::new(program).unwrap(),
        start(initial),
        &Limit {
            state: 4_096,
            terminal: 64,
            ..Limit::default()
        },
        |_| false,
    );
    let runtime = execution::explore(
        program,
        initial,
        known,
        bound(),
        photonic::execution::Bound {
            terminal: 64,
            ..photonic::execution::Bound::default()
        },
        |_| false,
    );
    let context = || {
        format!(
            "{}\n{}",
            crate::text::configuration(initial, known),
            crate::text::program(program, known)
        )
    };
    if !machine.complete() || !runtime.complete() {
        return;
    }
    assert_eq!(machine.state, runtime.state, "{}", context());
    let expected = machine
        .terminal
        .iter()
        .map(|state| state.observation().key(256).unwrap())
        .collect::<HashSet<_>>();
    let actual = runtime
        .terminal
        .iter()
        .map(|observation| observation.key(256).unwrap())
        .collect::<HashSet<_>>();
    assert_eq!(expected, actual, "{}", context());
}

#[test]
fn random() {
    let mut generator = Generator::new(21);
    let mut state = 0;
    let mut path = 0;
    for _ in 0..600 {
        let program = flat(&mut generator);
        let initial = configuration(&mut generator);
        let known = vocabulary();
        kernel(&program, &initial, &known);
        if !explore(
            &Flat::new(&program).unwrap(),
            start(&initial),
            &limit(),
            |_| false,
        )
        .complete()
        {
            continue;
        }
        state += exhaustive(&program, &initial, &known);
        path += usize::from(direct(&program, &initial, &known));
    }
    assert!(state > 300, "{state}");
    assert!(path > 100, "{path}");
}

#[test]
fn written() {
    let case = [
        "A.X, [A] (B, C), [B, C] D",
        "A.X, [A] (B, C), [B.X, C] D",
        "A.X, B.X, [A, B] C",
        "A, B, D, [A, ()] C",
        "A.B.C, [A] D, [B] E",
        "A.X, B, [A] ()",
        "A.X, B, [A],",
        "First.2, Second.0, [First.2, Second.0] (First.0, Second.2)",
    ];
    for source in case {
        let parsed = photonic::lowering::parse(source).unwrap();
        let mut known = Vocabulary::default();
        let (program, initial) = lift::program(&parsed, &mut known).unwrap();
        exhaustive(&program, &initial, &known);
        direct(&program, &initial, &known);
        kernel(&program, &initial, &known);
    }
}
