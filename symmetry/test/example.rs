use super::support::{named, single};
use crate::structure::Structure;
use code::atom::Atom;

const BUDGET: usize = 100_000;

fn size(structure: &Structure) -> String {
    structure
        .symmetry(BUDGET)
        .expect("examples fit the budget")
        .size
        .to_string()
}

fn cycle(length: usize, both: bool) -> Structure {
    let atom = (0..length)
        .map(|index| format!("A{index}"))
        .collect::<Vec<_>>();
    let mut rule = Vec::new();
    for index in 0..length {
        let next = (index + 1) % length;
        rule.push((
            vec![vec![atom[index].as_str()]],
            vec![vec![atom[next].as_str()]],
        ));
        if both {
            rule.push((
                vec![vec![atom[next].as_str()]],
                vec![vec![atom[index].as_str()]],
            ));
        }
    }
    single(named(&rule, &mut Vec::new()))
}

#[test]
fn cyclic() {
    assert_eq!(size(&cycle(7, false)), "7");
    assert_eq!(size(&cycle(7, true)), "14");
    assert_eq!(size(&cycle(12, true)), "24");
}

#[test]
fn light() {
    let mut name = Vec::new();
    let structure = single(named(
        &[
            (vec![vec!["Light"]], vec![vec!["Red"]]),
            (vec![vec!["Light"]], vec![vec!["Green"]]),
            (vec![vec!["Light"]], vec![vec!["Blue"]]),
        ],
        &mut name,
    ));
    let symmetry = structure.symmetry(BUDGET).expect("examples fit the budget");
    assert_eq!(symmetry.size.to_string(), "6");
    assert!(symmetry.block.is_empty());
}

#[test]
fn twin() {
    let mut name = Vec::new();
    let structure = single(named(
        &[
            (vec![vec!["A", "B"]], vec![vec!["C", "D", "E"]]),
            (vec![vec!["C", "D", "E"]], vec![vec!["F"]]),
        ],
        &mut name,
    ));
    let symmetry = structure.symmetry(BUDGET).expect("examples fit the budget");
    assert_eq!(symmetry.size.to_string(), "12");
    assert_eq!(symmetry.block.len(), 2);
    assert!(symmetry.generator.is_empty());
}

#[test]
fn duality() {
    let mut name = Vec::new();
    let and = single(named(
        &[
            (
                vec![vec!["Function", "Boolean", "And", "True", "True"]],
                vec![vec!["Return", "True"]],
            ),
            (
                vec![vec!["Function", "Boolean", "And", "True", "False"]],
                vec![vec!["Return", "False"]],
            ),
            (
                vec![vec!["Function", "Boolean", "And", "False", "False"]],
                vec![vec!["Return", "False"]],
            ),
        ],
        &mut name,
    ));
    let or = single(named(
        &[
            (
                vec![vec!["Function", "Boolean", "Or", "True", "True"]],
                vec![vec!["Return", "True"]],
            ),
            (
                vec![vec!["Function", "Boolean", "Or", "True", "False"]],
                vec![vec!["Return", "True"]],
            ),
            (
                vec![vec!["Function", "Boolean", "Or", "False", "False"]],
                vec![vec!["Return", "False"]],
            ),
        ],
        &mut name,
    ));
    let find =
        |text: &str| Atom(name.iter().position(|entry| entry == text).expect("named") as u16);
    let (left, right) = (
        and.symmetry(BUDGET).expect("examples fit the budget"),
        or.symmetry(BUDGET).expect("examples fit the budget"),
    );
    let map = left.isomorphism(&right).expect("And and Or are dual");
    assert_eq!(map[&find("True")], find("False"));
    assert_eq!(map[&find("False")], find("True"));
    let mut pinned = and;
    pinned.pin = vec![find("True")];
    let mut other = or;
    other.pin = vec![find("True")];
    assert!(
        pinned
            .symmetry(BUDGET)
            .expect("examples fit the budget")
            .isomorphism(&other.symmetry(BUDGET).expect("examples fit the budget"))
            .is_none()
    );
}

#[test]
fn pin() {
    let mut structure = cycle(9, true);
    structure.pin = vec![structure.atom()[0]];
    assert_eq!(size(&structure), "2");
    structure.pin.push(structure.atom()[1]);
    assert_eq!(size(&structure), "1");
}

fn graph(edge: &[(usize, usize)]) -> Structure {
    let name = (0..=edge
        .iter()
        .map(|&(from, to)| from.max(to))
        .max()
        .unwrap_or(0))
        .map(|index| format!("V{index}"))
        .collect::<Vec<_>>();
    let rule = edge
        .iter()
        .flat_map(|&(from, to)| [(from, to), (to, from)])
        .map(|(from, to)| {
            (
                vec![vec![name[from].as_str()]],
                vec![vec![name[to].as_str()]],
            )
        })
        .collect::<Vec<_>>();
    single(named(&rule, &mut Vec::new()))
}

#[test]
fn transitive() {
    let petersen = (0..5)
        .flat_map(|index| {
            [
                (index, (index + 1) % 5),
                (index, index + 5),
                (index + 5, (index + 2) % 5 + 5),
            ]
        })
        .collect::<Vec<_>>();
    assert_eq!(size(&graph(&petersen)), "120");
    let cube = (0..16usize)
        .flat_map(|index| {
            (0..4)
                .map(move |bit| (index, index ^ 1 << bit))
                .filter(|&(from, to)| from < to)
        })
        .collect::<Vec<_>>();
    assert_eq!(size(&graph(&cube)), "384");
    let bipartite = (0..3)
        .flat_map(|left| (3..7).map(move |right| (left, right)))
        .collect::<Vec<_>>();
    assert_eq!(size(&graph(&bipartite)), "144");
    let pair = (0..5)
        .flat_map(|index| [(index, (index + 1) % 5), (index + 5, (index + 1) % 5 + 5)])
        .collect::<Vec<_>>();
    assert_eq!(size(&graph(&pair)), "200");
    let complete = (0..9)
        .flat_map(|from| (from + 1..9).map(move |to| (from, to)))
        .collect::<Vec<_>>();
    assert_eq!(size(&graph(&complete)), "362880");
}
