use photonic::lowering::parse;
use photonic::obsidian::{Outcome, Search};
use photonic::runtime::Limit;

const APPLICATION: &str = include_str!("../application.particle");
const BOOLEAN: &str = include_str!("../boolean.particle");
const PRODUCTION: &str = include_str!("../production.particle");
const COLLECTION: &str = include_str!("../collection.particle");
const SELECTION: &str = include_str!("../selection.particle");
const TERNARY: &str = include_str!("../ternary.particle");
const CARRY: &str = include_str!("../carry.particle");
const COMPOSITION: &str = include_str!("../composition.particle");
const PAIR: &str = include_str!("../pair.particle");

fn program(source: &str, library: &[&str]) -> photonic::source::Program {
    parse(
        &std::iter::once(source)
            .chain(library.iter().copied())
            .collect::<Vec<_>>()
            .join("\n"),
    )
    .unwrap()
}

fn check(source: &str, target: &str, library: &[&str], expected: Outcome) {
    let mut search = Search::new(program(source, library), parse(target).unwrap()).unwrap();
    search.run(
        2_000_000,
        Some(Limit {
            state: 20000,
            record: 2_000_000,
            cell: 128,
            world: 32,
            frame: 32,
        }),
    );
    let report = search.report();
    assert!(
        report.execution.closed,
        "{source}: {} states, {} work",
        report.execution.state.len(),
        report.execution.work
    );
    assert_eq!(report.outcome, expected, "{source} => {target}");
}

fn witness(source: &str, target: &str, library: &[&str]) {
    let mut search =
        photonic::path::Search::new(program(source, library), parse(target).unwrap()).unwrap();
    search.run(
        2_000_000,
        Limit {
            state: 4096,
            record: 2_000_000,
            cell: 256,
            world: 64,
            frame: 64,
        },
    );
    let report = search.report();
    assert_eq!(
        report.outcome,
        Outcome::Reached,
        "{source} => {target}: {} work; {:?}",
        report.work,
        report.state.last()
    );
}

#[test]
fn application() {
    for value in ["True", "False"] {
        let expected = if value == "True" { "False" } else { "True" };
        check(
            &format!("Apply.Not.{value}"),
            expected,
            &[APPLICATION, BOOLEAN],
            Outcome::Reached,
        );
        check(
            &format!("Apply.Not.{value}"),
            value,
            &[APPLICATION, BOOLEAN],
            Outcome::Unreachable,
        );
    }
    check(
        "Apply.Identity.Payload",
        "Payload",
        &[APPLICATION],
        Outcome::Reached,
    );
}

#[test]
fn pipeline() {
    let source = include_str!("../../example/library/pipeline.wave");
    witness(
        source,
        include_str!("../../example/library/pipeline.particle"),
        &[APPLICATION, BOOLEAN, PRODUCTION, COLLECTION],
    );
}

#[test]
fn isolation() {
    check(
        "Apply.Not.True, Apply.Not.False",
        "False,True",
        &[APPLICATION, BOOLEAN],
        Outcome::Reached,
    );
    check(
        "Apply.Not.True, Apply.Not.False",
        "False,False",
        &[APPLICATION, BOOLEAN],
        Outcome::Unreachable,
    );
    check(
        "Apply.Not.True, Apply.Not.False",
        "True,True",
        &[APPLICATION, BOOLEAN],
        Outcome::Unreachable,
    );
}

#[test]
fn freshness() {
    check(
        "Call.Produce.([Value] True), Call.Produce.([Value] True)",
        "True.True",
        &[PRODUCTION, "[Return.True, Return.True] True.True"],
        Outcome::Reached,
    );
    check(
        "Call.Produce.([Value] True), Call.Produce.([Value] True)",
        "True",
        &[PRODUCTION, "[Return.True, Return.True] True.True"],
        Outcome::Unreachable,
    );
    check(
        "Call.Broadcast.True",
        "True",
        &[PRODUCTION, "[Ready.Left, Ready.Right] ()"],
        Outcome::Reached,
    );
    check(
        "Call.Broadcast.True",
        "True.True",
        &[PRODUCTION, "[Ready.Left, Ready.Right] ()"],
        Outcome::Unreachable,
    );
    witness(
        "Apply.Copy.Pair.([Value] True).Map.([Function] Identity).Gather",
        "([Left] True).([Right] True)",
        &[APPLICATION, PRODUCTION, COLLECTION],
    );
}

#[test]
fn repetition() {
    let source = "Apply.Repeat.Pair.([Value] 1).([Function] Produce).Reduce.([Operation] Add)";
    let library = [APPLICATION, PRODUCTION, COLLECTION, TERNARY];
    witness(source, "([Digit] 2).([Carry] 0)", &library);
}

#[test]
fn selection() {
    for (value, expected) in [("True", "2"), ("False", "0")] {
        let source = format!(
            "Apply.Copy.Pair.([Value] {value}).Map.([Function] Filter).Reduce.([Operation] Count)"
        );
        witness(
            &source,
            expected,
            &[APPLICATION, PRODUCTION, COLLECTION, SELECTION],
        );
    }
}

#[test]
fn order() {
    for left in ["True", "False"] {
        for right in ["True", "False"] {
            for condition in ["True", "False"] {
                let source = format!("Apply.Choose.{condition}.([Left] {left}).([Right] {right})");
                let expected = if condition == "True" { left } else { right };
                check(&source, expected, &[APPLICATION, PAIR], Outcome::Reached);
                check(
                    &source,
                    if expected == "True" { "False" } else { "True" },
                    &[APPLICATION, PAIR],
                    Outcome::Unreachable,
                );
            }
        }
    }
}

#[test]
fn arithmetic() {
    let digit = ["0", "1", "2"];
    for (left, first) in digit.iter().enumerate() {
        for (right, second) in digit.iter().enumerate() {
            for (operation, value) in [("Add", left + right), ("Multiply", left * right)] {
                let source = format!("Apply.{operation}.{first}.{second}");
                let target = format!(
                    "([Digit] {}).([Carry] {})",
                    digit[value % 3],
                    digit[value / 3]
                );
                check(&source, &target, &[APPLICATION, TERNARY], Outcome::Reached);
            }
            let source = format!("Apply.Compare.([Left] {first}).([Right] {second})");
            let expected = if left < right {
                "Less"
            } else if left == right {
                "Equal"
            } else {
                "Greater"
            };
            check(&source, expected, &[APPLICATION, TERNARY], Outcome::Reached);
        }
    }
}

#[test]
fn carry() {
    let state = ["Kill", "Propagate", "Generate"];
    for left in 0..3 {
        for right in 0..3 {
            let source = format!(
                "Apply.Carry.Compose.([Left] {}).([Right] {})",
                state[left], state[right]
            );
            let expected = if right == 1 { left } else { right };
            for (target, &value) in state.iter().enumerate() {
                check(
                    &source,
                    value,
                    &[APPLICATION, CARRY],
                    if target == expected {
                        Outcome::Reached
                    } else {
                        Outcome::Unreachable
                    },
                );
            }
        }
    }
}

#[test]
fn composition() {
    for first in ["Not", "Identity"] {
        for second in ["Not", "Identity"] {
            let source = format!("Apply.Compose.([First] {first}).([Second] {second}).True");
            let expected = if (first == "Not") == (second == "Not") {
                "True"
            } else {
                "False"
            };
            check(
                &source,
                expected,
                &[APPLICATION, BOOLEAN, COMPOSITION],
                Outcome::Reached,
            );
        }
    }
}

#[test]
fn function() {
    check(
        "Apply.True.([Call.True] Return.False)",
        "False.([Call.True] Return.False)",
        &[APPLICATION],
        Outcome::Reached,
    );
    check(
        "Apply.True.([Call.True] Return.False)",
        "True.([Call.True] Return.False)",
        &[APPLICATION],
        Outcome::Unreachable,
    );
}

#[test]
fn completion() {
    check(
        "Apply.Gather.Done.Left.True",
        "([Left] True).([Right] True)",
        &[APPLICATION, COLLECTION],
        Outcome::Unreachable,
    );
    check(
        "Apply.Gather.Done.Left.True, Apply.Gather.Done.Right.True",
        "([Left] True).([Right] True)",
        &[APPLICATION, COLLECTION],
        Outcome::Unreachable,
    );
    for left in ["True", "False"] {
        for right in ["True", "False"] {
            let source = format!(
                "Apply.Unpack.([Left] {left}).([Right] {right}).Map.([Function] Identity).Gather"
            );
            let target = format!("([Left] {left}).([Right] {right})");
            witness(&source, &target, &[APPLICATION, COLLECTION, PAIR]);
        }
    }
    check(
        "Call.Produce.([Value] True), Call.Produce.([Value] True)",
        "True",
        &[PRODUCTION, "[Return, Return] ()"],
        Outcome::Reached,
    );
}

#[test]
fn scheduling() {
    let source = "Apply.Not.True, Apply.Not.False";
    let mut baseline = None;
    for worker in [1, 2, 4] {
        let executor = photonic::executor::Executor::new(worker).unwrap();
        let mut search = Search::new(
            program(source, &[APPLICATION, BOOLEAN]),
            parse("False,True").unwrap(),
        )
        .unwrap();
        search.parallel(&executor, 1, None);
        assert!(!search.report().execution.closed);
        search.parallel(&executor, 100_000, None);
        let report = search.report();
        assert_eq!(report.outcome, Outcome::Reached);
        assert!(report.execution.closed);
        let actual = format!("{:?}", report.execution);
        if let Some(expected) = &baseline {
            assert_eq!(&actual, expected);
        }
        baseline = Some(actual);
    }
}

#[test]
fn rejection() {
    for first in ["Not", "Identity"] {
        for second in ["Not", "Identity"] {
            let source = format!("Apply.Compose.([First] {first}).([Second] {second}).True");
            let wrong = if (first == "Not") == (second == "Not") {
                "False"
            } else {
                "True"
            };
            check(
                &source,
                wrong,
                &[APPLICATION, BOOLEAN, COMPOSITION],
                Outcome::Unreachable,
            );
        }
    }
}

#[test]
fn declaration() {
    fn value(value: &photonic::source::Value) {
        match value {
            photonic::source::Value::Atom(atom) => {
                assert!(
                    atom.chars().all(char::is_alphabetic)
                        || atom.chars().all(|value| value.is_ascii_digit()),
                    "{atom}"
                );
                assert!(
                    atom.chars().all(|value| value.is_ascii_digit())
                        || atom.chars().skip(1).all(char::is_lowercase),
                    "{atom}"
                );
            }
            photonic::source::Value::Rule { rule: definition } => rule(definition),
        }
    }
    fn rule(definition: &photonic::source::Definition) {
        assert!(definition.input.len() <= 2);
        assert!(definition.output.len() <= 2);
        for item in definition.input.iter().flatten() {
            value(item);
        }
        for output in &definition.output {
            for item in &output.particle {
                value(item);
            }
            if let Some(body) = &output.body {
                for definition in body {
                    rule(definition);
                }
            }
        }
    }
    for source in [
        APPLICATION,
        BOOLEAN,
        PRODUCTION,
        COLLECTION,
        SELECTION,
        TERNARY,
        CARRY,
        PAIR,
        COMPOSITION,
        include_str!("../position.particle"),
        include_str!("../stream.particle"),
        include_str!("../binary.particle"),
    ] {
        let library = parse(source).unwrap();
        assert!(library.initial.is_empty());
        for definition in &library.rule {
            rule(definition);
        }
    }
}

#[test]
fn empty() {
    check(
        "Apply.Repeat.Empty",
        "()",
        &[APPLICATION, PRODUCTION],
        Outcome::Reached,
    );
    check(
        "Apply.Gather.Empty",
        "()",
        &[APPLICATION, COLLECTION],
        Outcome::Reached,
    );
    check(
        "Apply.Reduce.Empty.([Operation] And)",
        "True",
        &[APPLICATION, COLLECTION],
        Outcome::Reached,
    );
    check(
        "Apply.Reduce.Empty.([Operation] Or)",
        "False",
        &[APPLICATION, COLLECTION],
        Outcome::Reached,
    );
}

#[test]
fn association() {
    let source = "Apply.Pair.True.([Operation] And), Apply.Pair.False.([Operation] And)";
    let library = [
        APPLICATION,
        BOOLEAN,
        COLLECTION,
        "[Call.Pair.True] (Reduce.Done.Left.True, Reduce.Done.Right.True) [Call.Pair.False] (Reduce.Done.Left.False, Reduce.Done.Right.False)",
    ];
    check(source, "True,False", &library, Outcome::Reached);
    check(source, "True,True", &library, Outcome::Unreachable);
    check(source, "False,False", &library, Outcome::Unreachable);
}

#[test]
fn boolean() {
    let value = ["False", "True"];
    for left in 0..2 {
        for right in 0..2 {
            for (operation, result) in [
                ("And", left == 1 && right == 1),
                ("Or", left == 1 || right == 1),
                ("Equal", left == right),
            ] {
                let source = format!("Apply.{operation}.{}.{}", value[left], value[right]);
                for (index, &target) in value.iter().enumerate() {
                    check(
                        &source,
                        target,
                        &[APPLICATION, BOOLEAN],
                        if index == usize::from(result) {
                            Outcome::Reached
                        } else {
                            Outcome::Unreachable
                        },
                    );
                }
            }
        }
    }
}

#[test]
fn digit() {
    let digit = ["0", "1", "2"];
    for (left, first) in digit.iter().enumerate() {
        for (right, second) in digit.iter().enumerate() {
            for (operation, expected) in [("Add", left + right), ("Multiply", left * right)] {
                let source = format!("Apply.{operation}.{first}.{second}");
                for value in 0..9 {
                    let target = format!(
                        "([Digit] {}).([Carry] {})",
                        digit[value % 3],
                        digit[value / 3]
                    );
                    check(
                        &source,
                        &target,
                        &[APPLICATION, TERNARY],
                        if value == expected {
                            Outcome::Reached
                        } else {
                            Outcome::Unreachable
                        },
                    );
                }
            }
        }
        let source = format!("Apply.Successor.{first}");
        let value = left + 1;
        let target = format!(
            "([Digit] {}).([Carry] {})",
            digit[value % 3],
            digit[value / 3]
        );
        check(&source, &target, &[APPLICATION, TERNARY], Outcome::Reached);
    }
}

#[test]
fn evaluation() {
    for state in ["Kill", "Propagate", "Generate"] {
        for input in ["0", "1"] {
            let source = format!("Apply.Carry.Evaluate.{state}.{input}");
            let expected = match state {
                "Kill" => "0",
                "Generate" => "1",
                _ => input,
            };
            for target in ["0", "1"] {
                check(
                    &source,
                    target,
                    &[APPLICATION, CARRY],
                    if target == expected {
                        Outcome::Reached
                    } else {
                        Outcome::Unreachable
                    },
                );
            }
        }
    }
}

#[test]
fn position() {
    let position = include_str!("../position.particle");
    for index in 0..4 {
        for value in 0..3 {
            let source = format!("Apply.Unpack.([Position] {index}).([{index}] {value})");
            for expected in 0..3 {
                check(
                    &source,
                    &format!("([Value] {expected})"),
                    &[APPLICATION, position],
                    if expected == value {
                        Outcome::Reached
                    } else {
                        Outcome::Unreachable
                    },
                );
            }
            check(
                &format!("Apply.Pack.([Position] {index}).([Value] {value})"),
                &format!("([{index}] {value})"),
                &[APPLICATION, position],
                Outcome::Reached,
            );
        }
    }
    check(
        "Number.([Base] 3).([0] 2).([1] 0).([2] 2).([3] 1)",
        "Number.([3] 1).([1] 0).([0] 2).([Base] 3).([2] 2)",
        &[],
        Outcome::Reached,
    );
    check(
        "Number.([Base] 3).([0] 2).([1] 0)",
        "Number.([Base] 3).([0] 0).([1] 2)",
        &[],
        Outcome::Unreachable,
    );
    check(
        "Apply.Unpack.([Position] 0).([1] 2)",
        "([Value] 2)",
        &[APPLICATION, position],
        Outcome::Unreachable,
    );
}
