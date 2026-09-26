use super::support::{A, B, C, D, configuration, program, rule, state};
use crate::limit::Limit;
use crate::walk::walk;
use std::time::Instant;

#[test]
fn chain() {
    let program = program(vec![rule(&[&[A]], &[&[B]]), rule(&[&[B]], &[&[C]])]);
    let result = walk(&program, state(&[&[A], &[A]]), &Limit::default(), |_| 0);
    assert_eq!(result.work, 4);
    assert_eq!(result.depth, 2);
    assert_eq!(
        result.terminal.unwrap().configuration(),
        configuration(&[&[C], &[C]])
    );
}

#[test]
fn choice() {
    let program = program(vec![rule(&[&[A]], &[&[B]]), rule(&[&[A]], &[&[D]])]);
    let first = walk(&program, state(&[&[A]]), &Limit::default(), |_| 0);
    let last = walk(&program, state(&[&[A]]), &Limit::default(), |count| {
        count - 1
    });
    assert_ne!(
        first.terminal.unwrap().configuration(),
        last.terminal.unwrap().configuration()
    );
}

#[test]
fn cycle() {
    let program = program(vec![rule(&[&[A]], &[&[B]]), rule(&[&[B]], &[&[A]])]);
    let result = walk(&program, state(&[&[A]]), &Limit::default(), |_| 0);
    assert!(result.cycle);
    assert!(result.terminal.is_none());
}

#[test]
fn deadline() {
    let program = program(vec![rule(&[&[A]], &[&[B]])]);
    let expired = Limit {
        deadline: Some(Instant::now()),
        ..Limit::default()
    };
    let result = walk(&program, state(&[&[A]]), &expired, |_| 0);
    assert!(result.overflow && result.terminal.is_none());
}
