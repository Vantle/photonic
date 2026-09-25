use super::support::{A, B, C, D, configuration, program, rule, state};
use crate::limit::{Limit, Overflow};
use crate::schedule::schedule;

#[test]
fn parallel() {
    let program = program(vec![rule(&[&[A]], &[&[B]]), rule(&[&[B]], &[&[C]])]);
    let result = schedule(&program, state(&[&[A], &[A], &[A]]), &Limit::default()).unwrap();
    assert_eq!(result.round, vec![3, 3]);
    assert_eq!(result.work(), 6);
    assert_eq!(result.depth, 2);
    assert_eq!(
        result.terminal.configuration(),
        configuration(&[&[C], &[C], &[C]])
    );
}

#[test]
fn sequential() {
    let program = program(vec![rule(&[&[A]], &[&[B]]), rule(&[&[C]], &[&[D]])]);
    let result = schedule(&program, state(&[&[A, C]]), &Limit::default()).unwrap();
    assert_eq!(result.round, vec![1, 1]);
    assert_eq!(result.depth, 2);
    assert_eq!(result.terminal.configuration(), configuration(&[&[B, D]]));
}

#[test]
fn conflict() {
    let program = program(vec![rule(&[&[A], &[B]], &[&[C]])]);
    let result = schedule(
        &program,
        state(&[&[A], &[B], &[A], &[B]]),
        &Limit::default(),
    )
    .unwrap();
    assert_eq!(result.round, vec![2]);
    assert_eq!(result.depth, 1);
    assert_eq!(
        result.terminal.configuration(),
        configuration(&[&[C], &[C]])
    );
}

#[test]
fn divergence() {
    let program = program(vec![rule(&[&[A]], &[&[A]])]);
    assert_eq!(
        schedule(&program, state(&[&[A]]), &Limit::default()).unwrap_err(),
        Overflow::Round
    );
}
