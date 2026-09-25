use super::support::{Shape, Written, named, rule};
use crate::analysis::analyze;
use crate::statement::{Statement, structure};
use code::atom::Atom;
use code::forest::Forest;
use random::Generator;
use std::collections::{BTreeMap, BTreeSet};

const BUDGET: usize = 100_000;

fn planted(generator: &mut Generator) -> Vec<Statement> {
    let shape = Shape {
        atom: 6,
        rule: 1,
        depth: generator.below(2),
    };
    let motif = (0..1 + generator.below(3))
        .map(|_| rule(generator, &shape, 0))
        .collect::<Vec<_>>();
    let shared = generator.below(3);
    let mut statement = Vec::new();
    for copy in 0..2 + generator.below(3) {
        let map = |atom: Atom| {
            if usize::from(atom.0) < shared {
                return atom;
            }
            Atom(atom.0 + 16 * (copy as u16 + 1))
        };
        statement.extend(
            motif
                .iter()
                .map(|entry| Statement::Rule(entry.clone()).rename(map)),
        );
    }
    let noise = Shape {
        atom: 80,
        rule: 1,
        depth: 1,
    };
    for _ in 0..generator.below(4) {
        statement.push(Statement::Rule(rule(generator, &noise, 0)));
    }
    generator.shuffle(&mut statement);
    statement
}

#[test]
fn exact() {
    let mut generator = Generator::new(5);
    let mut found = 0;
    for _ in 0..300 {
        let statement = planted(&mut generator);
        let analysis =
            analyze(&structure(&statement), &statement, BUDGET).expect("fits the budget");
        let mut seen = BTreeSet::new();
        for pattern in &analysis.pattern {
            assert!(pattern.occurrence.len() > 1);
            for occurrence in &pattern.occurrence {
                assert_eq!(
                    occurrence.atom.iter().collect::<BTreeSet<_>>().len(),
                    occurrence.atom.len()
                );
                for &member in &occurrence.statement {
                    assert!(seen.insert(member), "statement {member} is in two copies");
                    assert!(
                        !analysis
                            .statement
                            .iter()
                            .flatten()
                            .any(|&moved| moved == member)
                    );
                }
            }
            let first = &pattern.occurrence[0];
            if first.statement.len() == 1 {
                let mut forest = Forest::new(pattern.occurrence.len());
                for (left, one) in pattern.occurrence.iter().enumerate() {
                    for (right, other) in pattern.occurrence.iter().enumerate() {
                        if one.atom.iter().any(|atom| other.atom.contains(atom)) {
                            forest.join(left, right);
                        }
                    }
                }
                assert_eq!(forest.group().len(), 1);
            }
            for other in &pattern.occurrence[1..] {
                let map = first
                    .atom
                    .iter()
                    .copied()
                    .zip(other.atom.iter().copied())
                    .collect::<BTreeMap<_, _>>();
                for (&from, &to) in first.statement.iter().zip(&other.statement) {
                    assert_eq!(
                        statement[from].rename(|atom| map[&atom]),
                        statement[to],
                        "{statement:?}"
                    );
                }
            }
            found += 1;
        }
    }
    assert!(found > 60, "only {found} patterns were found");
}

#[test]
fn alphabet() {
    let mut name = Vec::new();
    let mut written = Vec::new();
    for digit in ["Zero", "One", "Two"] {
        written.push(Written(vec![vec!["Drop", digit]], vec![vec!["Forget"]]));
        written.push(Written(
            vec![vec!["Yield", digit, "Wait"]],
            vec![vec!["Push", digit]],
        ));
    }
    written.push(Written(vec![vec!["Add", "Zero", "One"]], vec![vec!["One"]]));
    written.push(Written(vec![vec!["Add", "One", "One"]], vec![vec!["Two"]]));
    let program = named(&written, &mut name);
    let statement = program
        .rule()
        .iter()
        .cloned()
        .map(Statement::Rule)
        .collect::<Vec<_>>();
    let analysis = analyze(&structure(&statement), &statement, BUDGET).expect("fits the budget");
    assert!(analysis.statement.is_empty());
    let find =
        |text: &str| Atom(name.iter().position(|entry| entry == text).expect("named") as u16);
    let pattern = analysis
        .pattern
        .iter()
        .find(|pattern| pattern.occurrence[0].statement.len() == 2)
        .expect("the digit hooks repeat");
    assert_eq!(pattern.occurrence.len(), 3);
    let varying = pattern
        .varying()
        .into_iter()
        .map(|position| {
            pattern
                .occurrence
                .iter()
                .map(|occurrence| occurrence.atom[position])
                .collect::<BTreeSet<_>>()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        varying,
        vec![
            [find("Zero"), find("One"), find("Two")]
                .into_iter()
                .collect()
        ]
    );
}

#[test]
fn chain() {
    let mut name = Vec::new();
    let program = named(
        &[
            Written(vec![vec!["A"]], vec![vec!["B"]]),
            Written(vec![vec!["B"]], vec![vec!["C"]]),
        ],
        &mut name,
    );
    let statement = program
        .rule()
        .iter()
        .cloned()
        .map(Statement::Rule)
        .collect::<Vec<_>>();
    let analysis = analyze(&structure(&statement), &statement, BUDGET).expect("fits the budget");
    assert!(analysis.statement.is_empty());
    let [pattern] = analysis.pattern.as_slice() else {
        panic!("the two rules are one pattern");
    };
    let shared = Atom(name.iter().position(|entry| entry == "B").expect("named") as u16);
    let place = pattern
        .occurrence
        .iter()
        .map(|occurrence| occurrence.atom.iter().position(|&atom| atom == shared))
        .collect::<Vec<_>>();
    assert_eq!(place.len(), 2);
    assert!(place.iter().all(Option::is_some));
    assert_ne!(place[0], place[1]);
}
