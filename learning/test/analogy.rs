use super::support::{A, B, C, D, X, Y, rule};
use crate::analogy::{correspond, partition};
use code::atom::Atom;
use code::rule::Rule;

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

fn wide(pair: &[[u16; 2]]) -> Rule {
    let input = (0..4)
        .map(|part| (6 * part..6 * part + 6).collect::<Vec<u16>>())
        .collect::<Vec<_>>();
    rule(
        &input.iter().map(Vec::as_slice).collect::<Vec<_>>(),
        &pair.iter().map(<[u16; 2]>::as_slice).collect::<Vec<_>>(),
    )
}

#[test]
fn structure() {
    let paired = wide(
        &(0..6)
            .flat_map(|index| [[index, 6 + index], [12 + index, 18 + index]])
            .collect::<Vec<_>>(),
    );
    let ring = wide(
        &(0..3)
            .flat_map(|index| {
                [
                    [index, 6 + index],
                    [3 + index, 15 + index],
                    [9 + index, 21 + index],
                    [12 + index, 18 + index],
                ]
            })
            .collect::<Vec<_>>(),
    );
    assert!(correspond(&paired, &ring).is_none());
    assert!(correspond(&ring, &paired).is_none());
    assert!(correspond(&paired, &paired).is_some());
    assert_eq!(partition(&[&paired, &ring]), vec![vec![0], vec![1]]);
}
