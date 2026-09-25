use super::support::{A, B, C, D, X, Y, rule};
use crate::analogy::{correspond, partition};
use crate::atom::Atom;

#[test]
fn correspondence() {
    let near = rule(&[&[X, B], &[A, Y], &[B, D]], &[&[X, A], &[A, D], &[B, Y]]);
    let far = rule(&[&[X, C], &[B, Y], &[C, D]], &[&[X, B], &[B, D], &[C, Y]]);
    let forward = rule(&[&[X, B], &[A, D], &[B, Y]], &[&[X, C], &[A, D], &[B, Y]]);
    let map = correspond(&near, &far).unwrap();
    assert_eq!(map.atom[&Atom(X)], Atom(X));
    assert_eq!(map.atom[&Atom(A)], Atom(B));
    assert_eq!(map.atom[&Atom(B)], Atom(C));
    assert_eq!(map.atom[&Atom(Y)], Atom(Y));
    assert!(correspond(&near, &forward).is_none());
    assert_eq!(
        partition(&[&near, &forward, &far]),
        vec![vec![0, 2], vec![1]]
    );
}
