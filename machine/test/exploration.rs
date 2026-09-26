use super::support::{A, B, C, D, E, X, configuration, program, rule, state};
use crate::exploration::explore;
use crate::limit::Limit;
use crate::state::State;
use code::configuration::Configuration;
use std::time::{Duration, Instant};

fn terminal(exploration: &crate::exploration::Exploration) -> Vec<Configuration> {
    let mut value = exploration
        .terminal
        .iter()
        .map(State::configuration)
        .collect::<Vec<_>>();
    value.sort();
    value
}

#[test]
fn reunion() {
    let program = program(vec![
        rule(&[&[A]], &[&[B], &[C]]),
        rule(&[&[B], &[C]], &[&[D]]),
    ]);
    let exploration = explore(&program, state(&[&[A, X]]), &Limit::default(), |_| false);
    assert!(exploration.complete());
    assert_eq!(exploration.state, 3);
    assert_eq!(terminal(&exploration), vec![configuration(&[&[D, X]])]);
}

#[test]
fn sharing() {
    let program = program(vec![rule(&[&[A]], &[&[B], &[C]])]);
    let exploration = explore(&program, state(&[&[A, X]]), &Limit::default(), |_| false);
    assert_eq!(exploration.terminal.len(), 1);
    let shared = exploration.terminal[0].key(64).unwrap();
    let independent = state(&[&[B, X], &[C, X]]).key(64).unwrap();
    assert_ne!(shared, independent);
    assert_eq!(
        exploration.terminal[0].configuration(),
        configuration(&[&[B, X], &[C, X]])
    );
}

#[test]
fn diamond() {
    let program = program(vec![rule(&[&[A]], &[&[D]]), rule(&[&[B]], &[&[E]])]);
    let exploration = explore(&program, state(&[&[A, B, C]]), &Limit::default(), |_| false);
    assert!(exploration.complete());
    assert_eq!(exploration.state, 4);
    assert_eq!(terminal(&exploration), vec![configuration(&[&[C, D, E]])]);
}

#[test]
fn choice() {
    let program = program(vec![rule(&[&[A]], &[&[B]]), rule(&[&[A]], &[&[C]])]);
    let exploration = explore(&program, state(&[&[A]]), &Limit::default(), |_| false);
    assert!(exploration.complete());
    assert_eq!(
        terminal(&exploration),
        vec![configuration(&[&[B]]), configuration(&[&[C]])]
    );
}

#[test]
fn cycle() {
    let alternating = program(vec![rule(&[&[A]], &[&[B]]), rule(&[&[B]], &[&[A]])]);
    assert!(explore(&alternating, state(&[&[A]]), &Limit::default(), |_| false).cycle);
    let renewal = program(vec![rule(&[&[A]], &[&[A]])]);
    assert!(explore(&renewal, state(&[&[A]]), &Limit::default(), |_| false).cycle);
}

#[test]
fn growth() {
    let program = program(vec![rule(&[&[A]], &[&[A, A]])]);
    let exploration = explore(&program, state(&[&[A]]), &Limit::default(), |_| false);
    assert!(exploration.overflow);
    assert!(!exploration.complete());
}

#[test]
fn deletion() {
    let removal = program(vec![rule(&[&[A]], &[])]);
    let exploration = explore(&removal, state(&[&[A, X], &[B]]), &Limit::default(), |_| {
        false
    });
    assert_eq!(terminal(&exploration), vec![configuration(&[&[B]])]);
    let retention = program(vec![rule(&[&[A]], &[&[]])]);
    let exploration = explore(
        &retention,
        state(&[&[A, X], &[B]]),
        &Limit::default(),
        |_| false,
    );
    assert_eq!(terminal(&exploration), vec![configuration(&[&[B], &[X]])]);
}

#[test]
fn merge() {
    let program = program(vec![rule(&[&[A], &[B]], &[&[C]])]);
    let exploration = explore(
        &program,
        state(&[&[A, X], &[B, X]]),
        &Limit::default(),
        |_| false,
    );
    assert_eq!(terminal(&exploration), vec![configuration(&[&[C, X, X]])]);
}

#[test]
fn empty() {
    let program = program(vec![rule(&[&[A], &[]], &[&[C]])]);
    let exploration = explore(
        &program,
        state(&[&[A], &[B], &[D]]),
        &Limit::default(),
        |_| false,
    );
    let mut expected = vec![
        configuration(&[&[B, C], &[D]]),
        configuration(&[&[B], &[C, D]]),
    ];
    expected.sort();
    assert_eq!(terminal(&exploration), expected);
}

#[test]
fn consumption() {
    let split = program(vec![
        rule(&[&[A]], &[&[B], &[C]]),
        rule(&[&[B, X], &[C]], &[&[D]]),
    ]);
    let exploration = explore(&split, state(&[&[A, X]]), &Limit::default(), |_| false);
    assert_eq!(terminal(&exploration), vec![configuration(&[&[D, X]])]);
}

#[test]
fn joint() {
    let program = program(vec![
        rule(&[&[A], &[A, X]], &[&[C, D]]),
        rule(&[&[B]], &[&[C, X], &[X, X]]),
    ]);
    let exploration = explore(
        &program,
        state(&[&[A, A, B], &[B, C]]),
        &Limit::default(),
        |_| false,
    );
    assert!(exploration.complete());
    assert_eq!(exploration.state, 8);
}

#[test]
fn crowd() {
    let program = program(vec![rule(&[&[A], &[A]], &[&[B]])]);
    let crowd = state(&[&[A], &[A, C], &[A, D], &[A, E], &[A, X], &[A, B]]);
    let bounded = Limit {
        event: 8,
        ..Limit::default()
    };
    assert!(explore(&program, crowd.clone(), &bounded, |_| false).overflow);
    assert!(!explore(&program, crowd, &Limit::default(), |_| false).overflow);
}

#[test]
fn deadline() {
    let program = program(vec![rule(&[&[A]], &[&[B]]), rule(&[&[B]], &[&[C]])]);
    let expired = Limit {
        deadline: Some(Instant::now()),
        ..Limit::default()
    };
    assert!(explore(&program, state(&[&[A], &[A]]), &expired, |_| false).overflow);
    let later = Limit {
        deadline: Some(Instant::now() + Duration::from_secs(600)),
        ..Limit::default()
    };
    assert!(explore(&program, state(&[&[A], &[A]]), &later, |_| false).complete());
}
