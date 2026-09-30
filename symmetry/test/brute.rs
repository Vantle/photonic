use super::support::{Shape, closure, permutation, rename, shuffle, structure};
use crate::structure::{Part, Structure, image};
use code::atom::Atom;
use random::Generator;
use std::collections::{BTreeMap, BTreeSet};

const BUDGET: usize = 100_000;

fn renaming(from: &Structure, to: &Structure, map: &BTreeMap<Atom, Atom>) -> bool {
    if map
        .iter()
        .any(|(atom, name)| atom != name && (from.pin.contains(atom) || to.pin.contains(name)))
    {
        return false;
    }
    let apply = |part: &Part| image(part, &|atom| map[&atom]);
    apply(&from.program) == to.program && from.target.as_ref().map(apply) == to.target
}

fn bijection(from: &Structure, to: &Structure) -> Vec<BTreeMap<Atom, Atom>> {
    let domain = from.atom();
    let codomain = to.atom();
    if domain.len() != codomain.len() {
        return Vec::new();
    }
    permutation(&codomain)
        .into_iter()
        .map(|order| domain.iter().copied().zip(order).collect())
        .collect()
}

fn automorphism(structure: &Structure) -> usize {
    bijection(structure, structure)
        .iter()
        .filter(|map| renaming(structure, structure, map))
        .count()
}

fn isomorphic(left: &Structure, right: &Structure) -> bool {
    bijection(left, right)
        .iter()
        .any(|map| renaming(left, right, map))
}

fn bare(structure: &Structure) -> Structure {
    Structure {
        pin: Vec::new(),
        ..structure.clone()
    }
}

fn sample(generator: &mut Generator) -> Structure {
    let shape = Shape {
        atom: 2 + generator.below(5),
        rule: 1 + generator.below(4),
        depth: generator.below(3),
    };
    let program = structure(generator, &shape).program;
    let target = generator.chance(0.5).then(|| {
        let rule = generator.below(2);
        structure(generator, &Shape { rule, ..shape }).program
    });
    let mut pin = (0..shape.atom as u16).map(Atom).collect::<Vec<_>>();
    generator.shuffle(&mut pin);
    pin.truncate(generator.below(3));
    let base = Structure {
        program,
        target,
        pin,
    };
    if generator.chance(0.5) {
        return closure(generator, &base);
    }
    base
}

fn displace(generator: &mut Generator, structure: &Structure) -> Structure {
    let atom = structure
        .atom()
        .into_iter()
        .chain(structure.pin.iter().copied())
        .collect::<BTreeSet<_>>();
    let mut order = atom.iter().copied().collect::<Vec<_>>();
    generator.shuffle(&mut order);
    let map = atom.into_iter().zip(order).collect::<BTreeMap<_, _>>();
    Structure {
        pin: structure.pin.clone(),
        ..rename(structure, |atom| map[&atom])
    }
}

fn partner(generator: &mut Generator, left: &Structure) -> Structure {
    match generator.below(5) {
        0 | 1 => shuffle(generator, left),
        2 => displace(generator, left),
        3 => Structure {
            program: left.target.clone().unwrap_or_default(),
            target: Some(left.program.clone()),
            pin: left.pin.clone(),
        },
        _ => Structure {
            pin: left.pin.clone(),
            ..sample(generator)
        },
    }
}

// Disjoint copies of one small component, six atoms in all at most so that every bijection can
// still be tried, and then maybe a pin, a target naming one copy, or a rule joining two copies.
fn crowd(generator: &mut Generator) -> Structure {
    let shape = Shape {
        atom: 1 + generator.below(2),
        rule: 1 + generator.below(2),
        depth: generator.below(2),
    };
    let base = structure(generator, &shape);
    let own = base.atom();
    let count = 2 + generator.below(6 / own.len().max(1) - 1);
    let copy = (0..count)
        .map(|index| {
            let map = own
                .iter()
                .enumerate()
                .map(|(position, &atom)| (atom, Atom((index * own.len() + position) as u16)))
                .collect::<BTreeMap<_, _>>();
            rename(&base, |atom| map[&atom])
        })
        .collect::<Vec<_>>();
    let join = |part: &[&Part]| Part {
        program: code::program::Program::new(
            part.iter()
                .flat_map(|part| part.program.rule().iter().cloned())
                .collect(),
            part.iter()
                .flat_map(|part| part.program.scope().iter().cloned())
                .collect(),
        ),
        configuration: code::configuration::Configuration::from(
            part.iter()
                .flat_map(|part| part.configuration.coherence().iter().cloned())
                .collect::<Vec<_>>(),
        ),
    };
    let program = join(&copy.iter().map(|copy| &copy.program).collect::<Vec<_>>());
    let atom = (count * own.len()).max(1);
    match generator.below(4) {
        0 => Structure {
            program,
            target: None,
            pin: vec![Atom(generator.below(atom) as u16)],
        },
        1 => Structure {
            program,
            target: Some(copy[generator.below(count)].program.clone()),
            pin: Vec::new(),
        },
        2 => {
            let bridge = super::support::rule(
                generator,
                &Shape {
                    atom,
                    rule: 1,
                    depth: 0,
                },
                0,
            );
            let extra = Part {
                program: code::program::Program::from(vec![bridge]),
                configuration: code::configuration::Configuration::default(),
            };
            Structure {
                program: join(&[&program, &extra]),
                target: None,
                pin: Vec::new(),
            }
        }
        _ => Structure {
            program,
            target: None,
            pin: Vec::new(),
        },
    }
}

fn order(structure: &Structure) -> usize {
    let symmetry = structure
        .symmetry(BUDGET)
        .expect("small structures fit the budget");
    let count = automorphism(structure);
    assert_eq!(
        symmetry.size.to_string(),
        count.to_string(),
        "{structure:?}"
    );
    let atom = structure.atom();
    for permutation in super::support::generator(&symmetry) {
        let map = atom
            .iter()
            .map(|&atom| (atom, permutation.image(atom)))
            .collect();
        assert!(renaming(structure, structure, &map), "{structure:?}");
    }
    count
}

fn decision(left: &Structure, right: &Structure) -> bool {
    let expected = isomorphic(left, right);
    let (first, second) = (
        left.symmetry(BUDGET)
            .expect("small structures fit the budget"),
        right
            .symmetry(BUDGET)
            .expect("small structures fit the budget"),
    );
    assert_eq!(first.key == second.key, expected, "{left:?}\n{right:?}");
    let map = first.isomorphism(&second);
    assert_eq!(map.is_some(), expected, "{left:?}\n{right:?}");
    let Some(map) = map else {
        return false;
    };
    assert!(renaming(left, right, &map), "{left:?}\n{right:?}");
    assert_eq!(first.form, second.form, "{left:?}\n{right:?}");
    true
}

#[test]
fn group() {
    let mut generator = Generator::new(7);
    let mut symmetric = 0;
    let mut pinned = 0;
    for _ in 0..1000 {
        let structure = sample(&mut generator);
        let count = order(&structure);
        symmetric += usize::from(count > 1);
        pinned += usize::from(automorphism(&bare(&structure)) > count);
    }
    assert!(symmetric > 100, "only {symmetric} samples were symmetric");
    assert!(pinned > 15, "pins cut the group of only {pinned} samples");
}

#[test]
fn isomorphism() {
    let mut generator = Generator::new(11);
    let mut equal = 0;
    let mut decided = 0;
    for _ in 0..600 {
        let left = sample(&mut generator);
        let right = partner(&mut generator, &left);
        if decision(&left, &right) {
            equal += 1;
            continue;
        }
        decided += usize::from(isomorphic(&bare(&left), &bare(&right)));
    }
    assert!(equal > 100, "only {equal} pairs were isomorphic");
    assert!(decided > 30, "pins decided only {decided} pairs");
}

#[test]
fn component() {
    let mut generator = Generator::new(13);
    let mut large = 0;
    let mut equal = 0;
    for _ in 0..400 {
        let structure = crowd(&mut generator);
        large += usize::from(order(&structure) >= 24);
        let right = partner(&mut generator, &structure);
        equal += usize::from(decision(&structure, &right));
    }
    assert!(
        large > 60,
        "only {large} samples had 24 automorphisms or more"
    );
    assert!(equal > 100, "only {equal} pairs were isomorphic");
}
