use super::support::{Shape, closure, permutation, shuffle, structure};
use crate::structure::Structure;
use random::Generator;
use std::collections::BTreeMap;

const BUDGET: usize = 100_000;

fn automorphism(structure: &Structure) -> usize {
    let atom = structure.atom();
    permutation(&atom)
        .into_iter()
        .filter(|image| {
            let map = atom
                .iter()
                .copied()
                .zip(image.iter().copied())
                .collect::<BTreeMap<_, _>>();
            super::support::rename(structure, |atom| map[&atom]) == *structure
        })
        .count()
}

fn isomorphic(left: &Structure, right: &Structure) -> bool {
    let (from, to) = (left.atom(), right.atom());
    if from.len() != to.len() {
        return false;
    }
    permutation(&to).into_iter().any(|image| {
        let map = from.iter().copied().zip(image).collect::<BTreeMap<_, _>>();
        super::support::rename(left, |atom| map[&atom]) == *right
    })
}

fn sample(generator: &mut Generator) -> Structure {
    let shape = Shape {
        atom: 2 + generator.below(5),
        rule: 1 + generator.below(4),
        depth: generator.below(3),
    };
    let base = structure(generator, &shape);
    if generator.chance(0.5) {
        return closure(generator, &base);
    }
    base
}

#[test]
fn order() {
    let mut generator = Generator::new(7);
    let mut symmetric = 0;
    for _ in 0..400 {
        let structure = sample(&mut generator);
        if structure.atom().len() > 7 {
            continue;
        }
        let symmetry = structure
            .symmetry(BUDGET)
            .expect("small structures fit the budget");
        let count = automorphism(&structure);
        assert_eq!(
            symmetry.size.to_string(),
            count.to_string(),
            "{structure:?}"
        );
        for permutation in super::support::generator(&symmetry) {
            assert_eq!(
                super::support::rename(&structure, |atom| permutation.image(atom)),
                structure
            );
        }
        symmetric += usize::from(count > 1);
    }
    assert!(symmetric > 50, "only {symmetric} samples were symmetric");
}

#[test]
fn decision() {
    let mut generator = Generator::new(11);
    let mut equal = 0;
    for _ in 0..600 {
        let left = sample(&mut generator);
        let right = if generator.chance(0.4) {
            shuffle(&mut generator, &left).0
        } else {
            sample(&mut generator)
        };
        if left.atom().len() > 6 || right.atom().len() > 6 {
            continue;
        }
        let expected = isomorphic(&left, &right);
        let (first, second) = (
            left.symmetry(BUDGET)
                .expect("small structures fit the budget"),
            right
                .symmetry(BUDGET)
                .expect("small structures fit the budget"),
        );
        let map = first.isomorphism(&second);
        assert_eq!(map.is_some(), expected, "{left:?}\n{right:?}");
        if let Some(map) = map {
            assert_eq!(super::support::rename(&left, |atom| map[&atom]), right);
            equal += 1;
        }
    }
    assert!(equal > 100, "only {equal} pairs were isomorphic");
}
