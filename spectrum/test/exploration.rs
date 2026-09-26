use super::support::{BUG, FIX, ORIGINAL, explore};
use crate::cause::Role;
use crate::claim::{self, Answer, Claim, Kind};
use crate::exploration::Value;
use crate::lineage;
use crate::recording::Order;
use crate::render;

fn claim(kind: Kind, pattern: &str) -> Claim {
    Claim {
        kind,
        pattern: pattern.to_owned(),
        exact: false,
        preserve: false,
    }
}

#[test]
fn size() {
    let bug = explore(BUG);
    assert_eq!(bug.order, Order::Shape);
    assert_eq!(bug.shape, Some(0x111d_39d9_6ed3_2425));
    assert!(bug.closed);
    assert_eq!(bug.configuration.len(), 18);
    assert_eq!(bug.event.len(), 25);
    assert_eq!(
        (0..bug.event.len())
            .filter(|&index| bug.inferred(index))
            .count(),
        6
    );
    assert_eq!(bug.work, 349);
    let original = explore(ORIGINAL);
    assert_eq!(
        (
            original.configuration.len(),
            original.event.len(),
            original.work
        ),
        (14, 17, 240)
    );
    let fix = explore(FIX);
    assert_eq!(
        (fix.configuration.len(), fix.event.len(), fix.work),
        (14, 18, 246)
    );
}

#[test]
fn handle() {
    let bug = explore(BUG);
    assert_eq!(render::configuration(&bug, 11), "False.True.Extra");
    assert_eq!(render::configuration(&bug, 8), "False.Extra");
    assert_eq!(render::configuration(&bug, 10), "in f1: False.True.Extra");
    assert_eq!(bug.rule[2].text, "[False] False");
    assert_eq!(bug.rule[2].scope, Some(1));
    assert_eq!(bug.event[12].rule, 2);
    assert_eq!((bug.event[12].source, bug.event[12].target), (10, 11));
    assert_eq!(bug.path(11), Some(vec![11, 12]));
    assert_eq!(bug.event[11].deduction, vec![0, 2]);
}

#[test]
fn evaluation() {
    let bug = explore(BUG);
    let reach = claim::evaluate(&claim(Kind::Reach, "False.Extra"), &bug).unwrap();
    assert_eq!(reach.answer, Answer::Holds);
    assert_eq!(reach.witness.as_deref(), Some("s0"));
    assert!(reach.path.is_empty());
    let exact = claim::evaluate(
        &Claim {
            exact: true,
            preserve: true,
            ..claim(Kind::Reach, "False.Extra")
        },
        &bug,
    )
    .unwrap();
    assert_eq!(exact.answer, Answer::Holds);
    assert_eq!(exact.witness.as_deref(), Some("s8"));
    assert_eq!(exact.path, vec!["e1", "e7", "e8"]);
    let absent = claim::evaluate(&claim(Kind::Reach, "Nothing"), &bug).unwrap();
    assert_eq!(absent.answer, Answer::Fails);
    let avoid = claim::evaluate(&claim(Kind::Avoid, "False.True.Extra"), &bug).unwrap();
    assert_eq!(avoid.answer, Answer::Fails);
    assert_eq!(avoid.witness.as_deref(), Some("s0"));
    let always = claim::evaluate(&claim(Kind::Always, "Extra"), &bug).unwrap();
    assert_eq!(always.answer, Answer::Holds);
    let outcome = claim::evaluate(&claim(Kind::Outcome, "Boolean"), &bug).unwrap();
    assert_eq!(outcome.answer, Answer::Fails);
    let inevitable = claim::evaluate(&claim(Kind::Inevitable, "Extra"), &bug).unwrap();
    assert_eq!(inevitable.answer, Answer::Holds);
    let stuck = claim::evaluate(&claim(Kind::Inevitable, "Boolean.Boolean"), &bug).unwrap();
    assert_eq!(stuck.answer, Answer::Fails);
}

#[test]
fn lineage() {
    let bug = explore(BUG);
    let line = lineage::lineage(&bug, 11, 1).unwrap();
    let role = line.iter().map(|line| line.role).collect::<Vec<_>>();
    assert_eq!(role, vec![Role::Remainder, Role::Witness, Role::Initial]);
    assert_eq!(line[0].event, Some(12));
    assert_eq!(line[1].event, Some(11));
    assert_eq!((line[2].configuration, line[2].occurrence), (0, 2));
    let original = explore(ORIGINAL);
    let produced = lineage::lineage(&original, 9, 0).unwrap();
    assert_eq!(produced.len(), 1);
    assert_eq!(produced[0].role, Role::Produced);
    assert_eq!(produced[0].event, Some(10));
    assert_eq!(produced[0].source.len(), 3);
    let held = lineage::lineage(&original, 8, 0).unwrap();
    let role = held.iter().map(|line| line.role).collect::<Vec<_>>();
    assert_eq!(role, vec![Role::Held, Role::Initial]);
}

#[test]
fn invariance() {
    let bug = explore(BUG);
    let renamed = explore(&BUG.replace("Boolean", "Bit").replace("Extra", "Rest"));
    assert_ne!(bug.key, renamed.key);
    assert_eq!(bug.shape, renamed.shape);
    assert_eq!(bug.event.len(), renamed.event.len());
    assert_eq!(render::configuration(&renamed, 11), "False.True.Rest");
    let reordered = explore(
        "[And.Boolean.Boolean] (\n    [False] False,\n    [True.True] True,\n),\n[False] Boolean,\n[True] Boolean,\nAnd.True.False.Extra\n",
    );
    assert_eq!(bug.key, reordered.key);
    assert_eq!(render::configuration(&reordered, 11), "False.True.Extra");
}

#[test]
fn symmetry() {
    for order in [
        [
            "Light, [Light] Red, [Light] Green, [Light] Blue",
            "Light, [Light] Blue, [Light] Green, [Light] Red",
            "[Light] Green, Light, [Light] Red, [Light] Blue",
        ],
        [
            "A.B, [A] C, [B] D",
            "[B] D, [A] C, B.A",
            "[A] C, B.A, [B] D",
        ],
    ] {
        let first = explore(order[0]);
        for source in &order[1..] {
            let other = explore(source);
            assert_eq!(first.key, other.key, "{source}");
            for (index, rule) in first.rule.iter().enumerate() {
                assert_eq!(rule.text, other.rule[index].text, "{source}");
            }
            for index in 0..first.configuration.len() {
                assert_eq!(
                    render::configuration(&first, index),
                    render::configuration(&other, index),
                    "{source}"
                );
            }
        }
    }
}

#[test]
fn opener() {
    let exploration = explore("A, B, [A] ([X] Y), [B] ([P] Q)");
    let mut seen = 0;
    for frame in exploration
        .configuration
        .iter()
        .flat_map(|configuration| configuration.frame.iter().skip(1))
    {
        let opener = frame.opener.expect("every scope has an opener");
        let body = exploration.rule[opener]
            .definition
            .output
            .iter()
            .flat_map(|output| match output {
                frontend::source::Output::Scope(program) => program.rule.as_slice(),
                frontend::source::Output::Particle(_) => &[],
            })
            .map(frontend::text::definition)
            .collect::<Vec<_>>();
        for occurrence in &frame.rule {
            let Value::Rule(rule) = occurrence.value else {
                continue;
            };
            assert!(
                body.contains(&exploration.rule[rule].text),
                "{} is not in the scope {} opens",
                exploration.rule[rule].text,
                exploration.rule[opener].text
            );
            assert_eq!(exploration.rule[rule].scope, Some(opener));
            seen += 1;
        }
    }
    assert!(seen >= 2);
}

#[test]
fn inevitable() {
    let exploration = explore("A, [A] M, [A] N, [M] E, [N] P, [P] E");
    let verdict = claim::evaluate(&claim(Kind::Inevitable, "M"), &exploration).unwrap();
    assert_eq!(verdict.answer, Answer::Fails);
    assert!(!verdict.path.is_empty());
    let mut cursor = 0;
    for handle in &verdict.path {
        let event = &exploration.event[handle[1..].parse::<usize>().unwrap()];
        assert_eq!(event.source, cursor);
        cursor = event.target;
        assert_ne!(render::configuration(&exploration, cursor), "M");
    }
    let witness = verdict.witness.expect("a witness");
    assert_eq!(witness, format!("s{cursor}"));
    assert_eq!(render::configuration(&exploration, cursor), "E");
}
