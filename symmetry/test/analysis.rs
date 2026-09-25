use super::support::named;
use crate::analysis::{Kind, analyze};
use crate::statement::{Statement, structure};
use std::collections::BTreeSet;

#[test]
fn category() {
    let mut name = Vec::new();
    let program = named(
        &[
            (vec![vec!["Light"]], vec![vec!["Red"]]),
            (vec![vec!["Light"]], vec![vec!["Green"]]),
            (vec![vec!["Light"]], vec![vec!["Blue"]]),
            (vec![vec!["Not", "True"]], vec![vec!["False"]]),
            (vec![vec!["Not", "False"]], vec![vec!["True"]]),
            (vec![vec!["Start"]], vec![vec!["Not", "True", "Boolean"]]),
            (vec![vec!["Go", "Fast"]], vec![vec!["Stop"]]),
        ],
        &mut name,
    );
    let statement = program
        .rule()
        .iter()
        .cloned()
        .map(Statement::Rule)
        .collect::<Vec<_>>();
    let analysis = analyze(&structure(&statement), &statement, 100_000).expect("fits the budget");
    let class = analysis.class(&statement);
    let set = |part: &[code::atom::Atom]| {
        part.iter()
            .map(|atom| name[atom.index()].as_str())
            .collect::<BTreeSet<_>>()
    };
    assert_eq!(
        class.iter().map(|class| class.kind).collect::<Vec<_>>(),
        [Kind::Global, Kind::Local, Kind::Block]
    );
    assert_eq!(class[0].part.len(), 1);
    assert_eq!(
        set(&class[0].part[0]),
        BTreeSet::from(["Blue", "Green", "Red"])
    );
    assert_eq!(class[0].statement.len(), 3);
    assert_eq!(class[1].part.len(), 2);
    assert_ne!(class[1].part[0], class[1].part[1]);
    assert!(
        class[1]
            .part
            .iter()
            .all(|part| set(part) == BTreeSet::from(["False", "True"]))
    );
    assert_eq!(class[1].statement.len(), 2);
    assert_eq!(set(&class[2].part[0]), BTreeSet::from(["Fast", "Go"]));
    assert!(class[2].statement.is_empty());
}
