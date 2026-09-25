use super::support::{named, single};
use crate::comparison::compare;
use code::atom::Atom;

const BUDGET: usize = 100_000;

#[test]
fn class() {
    let mut name = Vec::new();
    let and = single(named(
        &[
            (
                vec![vec!["Boolean", "And", "True", "True"]],
                vec![vec!["True"]],
            ),
            (
                vec![vec!["Boolean", "And", "True", "False"]],
                vec![vec!["False"]],
            ),
            (
                vec![vec!["Boolean", "And", "False", "False"]],
                vec![vec!["False"]],
            ),
        ],
        &mut name,
    ));
    let or = single(named(
        &[
            (
                vec![vec!["Boolean", "Or", "True", "True"]],
                vec![vec!["True"]],
            ),
            (
                vec![vec!["Boolean", "Or", "True", "False"]],
                vec![vec!["True"]],
            ),
            (
                vec![vec!["Boolean", "Or", "False", "False"]],
                vec![vec!["False"]],
            ),
        ],
        &mut name,
    ));
    let equal = single(named(
        &[
            (
                vec![vec!["Boolean", "Equal", "True", "True"]],
                vec![vec!["True"]],
            ),
            (
                vec![vec!["Boolean", "Equal", "True", "False"]],
                vec![vec!["False"]],
            ),
            (
                vec![vec!["Boolean", "Equal", "False", "False"]],
                vec![vec!["True"]],
            ),
        ],
        &mut name,
    ));
    let find =
        |text: &str| Atom(name.iter().position(|entry| entry == text).expect("named") as u16);
    let class = compare(&[and.clone(), equal, or.clone()], BUDGET).expect("fits the budget");
    assert_eq!(
        class
            .iter()
            .map(|class| class.member.clone())
            .collect::<Vec<_>>(),
        vec![vec![0, 2], vec![1]]
    );
    let row = &class[0].atom;
    assert!(row.contains(&vec![find("True"), find("False")]));
    assert!(row.contains(&vec![find("False"), find("True")]));
    assert!(row.contains(&vec![find("Boolean"), find("Boolean")]));
    assert!(row.contains(&vec![find("And"), find("Or")]));
    let map = row
        .iter()
        .map(|pair| (pair[0], pair[1]))
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(and.rename(|atom| map[&atom]), or);
    assert_eq!(class[0].block.len(), 1);
    assert_eq!(class[0].size.to_string(), "2");
    let canonical = class[0].form.atom();
    assert!(
        class[0].block[0]
            .iter()
            .all(|atom| canonical.contains(atom))
    );
    assert_eq!(class[1].atom.len(), 4);
    assert!(class[1].generator.is_empty());
}
