use super::support::{A, B, C, X, Y};
use crate::canonical::{Exhausted, Key, key};

fn shared(coherence: &[&[(u16, u32)]]) -> Key<u16> {
    key(
        &coherence
            .iter()
            .map(|entry| entry.iter().map(|&(atom, id)| (id, atom)).collect())
            .collect::<Vec<Vec<_>>>(),
        64,
    )
    .unwrap()
}

fn plain(coherence: &[&[u16]]) -> Key<u16> {
    let mut id = 0;
    key(
        &coherence
            .iter()
            .map(|entry| {
                entry
                    .iter()
                    .map(|&atom| {
                        id += 1;
                        (id, atom)
                    })
                    .collect()
            })
            .collect::<Vec<Vec<_>>>(),
        64,
    )
    .unwrap()
}

#[test]
fn order() {
    assert_eq!(plain(&[&[A, B], &[C]]), plain(&[&[C], &[B, A]]));
    assert_ne!(plain(&[&[A, B], &[C]]), plain(&[&[A], &[B, C]]));
    assert_ne!(plain(&[&[A]]), plain(&[&[A], &[]]));
}

#[test]
fn renaming() {
    let left = shared(&[&[(A, 1), (X, 7)], &[(B, 2), (X, 7)], &[(C, 3)]]);
    let right = shared(&[&[(C, 9)], &[(B, 4), (X, 5)], &[(X, 5), (A, 6)]]);
    assert_eq!(left, right);
    assert_ne!(left, plain(&[&[A, X], &[B, X], &[C]]));
}

#[test]
fn incidence() {
    let together = shared(&[&[(A, 1), (X, 7), (Y, 8)], &[(A, 2), (X, 7), (Y, 8)]]);
    let crossed = shared(&[
        &[(A, 1), (X, 7), (Y, 8)],
        &[(A, 2), (X, 7), (Y, 9)],
        &[(Y, 9), (Y, 8)],
    ]);
    let apart = shared(&[
        &[(A, 1), (X, 7), (Y, 9)],
        &[(A, 2), (X, 7), (Y, 8)],
        &[(Y, 8), (Y, 9)],
    ]);
    assert_eq!(crossed, apart);
    assert_ne!(together, crossed);
}

#[test]
fn symmetry() {
    let line = |offset: u32| {
        shared(&[
            &[(A, offset), (X, offset + 10)],
            &[(A, offset + 1), (X, offset + 10), (X, offset + 11)],
            &[(A, offset + 2), (X, offset + 11), (X, offset + 12)],
            &[(A, offset + 3), (X, offset + 12)],
        ])
    };
    let chain = shared(&[
        &[(A, 0), (X, 10)],
        &[(A, 1), (X, 11), (X, 12)],
        &[(A, 2), (X, 10), (X, 11)],
        &[(A, 3), (X, 12)],
    ]);
    assert_eq!(line(0), line(40));
    assert_eq!(line(0), chain);
    let star = shared(&[
        &[(A, 0), (X, 10), (X, 11), (X, 12)],
        &[(A, 1), (X, 10)],
        &[(A, 2), (X, 11)],
        &[(A, 3), (X, 12)],
    ]);
    assert_ne!(line(0), star);
}

fn relabel(coherence: &[Vec<(u32, u16)>], order: &[usize]) -> Vec<Vec<(u32, u16)>> {
    order
        .iter()
        .map(|&index| {
            coherence[index]
                .iter()
                .rev()
                .map(|&(id, atom)| (id + 1_000, atom))
                .collect()
        })
        .collect()
}

fn permutation(length: usize) -> [Vec<usize>; 3] {
    [
        (0..length).rev().collect(),
        (0..length).map(|index| (index + 5) % length).collect(),
        (0..length)
            .step_by(2)
            .chain((1..length).step_by(2))
            .collect(),
    ]
}

#[test]
fn orbit() {
    let star = std::iter::once(
        (0..12)
            .map(|index| (100 + index, X))
            .chain([(0, A)])
            .collect(),
    )
    .chain((0..12).map(|index| vec![(1 + index, B), (100 + index, X)]))
    .collect::<Vec<_>>();
    let component = (0..8)
        .flat_map(|index| {
            let base = 10 * index;
            [
                vec![(base, A), (base + 1, X)],
                vec![(base + 2, B), (base + 1, X), (base + 3, Y)],
                vec![(base + 4, C), (base + 3, Y)],
            ]
        })
        .collect::<Vec<_>>();
    for graph in [star, component] {
        let expected = key(&graph, 4_096).unwrap();
        for order in permutation(graph.len()) {
            assert_eq!(key(&relabel(&graph, &order), 4_096).unwrap(), expected);
        }
    }
}

#[test]
fn ring() {
    let ring = |length: u32, atom: &dyn Fn(u32) -> u16| {
        (0..length)
            .map(|index| {
                vec![
                    (index, atom(index)),
                    (100 + index, X),
                    (100 + (index + 1) % length, X),
                ]
            })
            .collect::<Vec<_>>()
    };
    let uniform = ring(6, &|_| A);
    let alternating = ring(6, &|index| if index % 2 == 0 { A } else { B });
    let paired = ring(6, &|index| if index % 3 == 0 { A } else { B });
    let expected = [&uniform, &alternating, &paired].map(|graph| key(graph, 4_096).unwrap());
    for (graph, expected) in [&uniform, &alternating, &paired].iter().zip(&expected) {
        for order in permutation(graph.len()) {
            assert_eq!(key(&relabel(graph, &order), 4_096).unwrap(), *expected);
        }
    }
    assert_ne!(expected[0], expected[1]);
    assert_ne!(expected[1], expected[2]);
    let split = [ring(3, &|_| A), ring(3, &|_| A)]
        .into_iter()
        .enumerate()
        .flat_map(|(copy, graph)| {
            graph.into_iter().map(move |coherence| {
                coherence
                    .into_iter()
                    .map(|(id, atom)| (id + 10_000 * copy as u32, atom))
                    .collect::<Vec<_>>()
            })
        })
        .collect::<Vec<_>>();
    assert_ne!(key(&split, 4_096).unwrap(), expected[0]);
    assert_eq!(key(&uniform, 0), Err(Exhausted));
}
