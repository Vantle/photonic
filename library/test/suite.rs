use photonic::lowering::parse;
use photonic::prism::{Outcome, Search};
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
    let mut search = {
        let program = program(source, library);
        let target = photonic::source::Program {
            rule: program.rule.clone(),
            ..parse(target).unwrap()
        };
        Search::new(program, target)
    };
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
    let mut search = {
        let program = program(source, library);
        let target = photonic::source::Program {
            rule: program.rule.clone(),
            ..parse(target).unwrap()
        };
        photonic::path::Search::new(program, target)
    };
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
            &format!("Invoke.Not.{value}"),
            expected,
            &[APPLICATION, BOOLEAN],
            Outcome::Reached,
        );
        check(
            &format!("Invoke.Not.{value}"),
            value,
            &[APPLICATION, BOOLEAN],
            Outcome::Unreachable,
        );
    }
    check(
        "Invoke.Identity.Payload",
        "Payload",
        &[APPLICATION],
        Outcome::Reached,
    );
}

#[test]
fn pipeline() {
    let source = include_str!("../../program/composition/pipeline.wave");
    witness(
        source,
        include_str!("../../program/composition/pipeline.particle"),
        &[APPLICATION, BOOLEAN, PRODUCTION, COLLECTION],
    );
}

#[test]
fn isolation() {
    check(
        "Invoke.Not.True, Invoke.Not.False",
        "False,True",
        &[APPLICATION, BOOLEAN],
        Outcome::Reached,
    );
    check(
        "Invoke.Not.True, Invoke.Not.False",
        "False,False",
        &[APPLICATION, BOOLEAN],
        Outcome::Unreachable,
    );
    check(
        "Invoke.Not.True, Invoke.Not.False",
        "True,True",
        &[APPLICATION, BOOLEAN],
        Outcome::Unreachable,
    );
}

#[test]
fn freshness() {
    check(
        "Function.Produce.([Value] True), Function.Produce.([Value] True)",
        "True.True",
        &[PRODUCTION, "[Return.True, Return.True] True.True"],
        Outcome::Reached,
    );
    check(
        "Function.Produce.([Value] True), Function.Produce.([Value] True)",
        "True",
        &[PRODUCTION, "[Return.True, Return.True] True.True"],
        Outcome::Unreachable,
    );
    check(
        "Function.Broadcast.True",
        "True",
        &[PRODUCTION, "[Ready.Left, Ready.Right] ()"],
        Outcome::Reached,
    );
    check(
        "Function.Broadcast.True",
        "True.True",
        &[PRODUCTION, "[Ready.Left, Ready.Right] ()"],
        Outcome::Unreachable,
    );
    witness(
        "Invoke.Copy.Pair.([Value] True).Map.([Each] Identity).Gather",
        "([Left] True).([Right] True)",
        &[APPLICATION, PRODUCTION, COLLECTION],
    );
}

#[test]
fn repetition() {
    let source = "Invoke.Repeat.Pair.([Value] 1).([Each] Produce).Reduce.([Operation] Add)";
    let library = [APPLICATION, PRODUCTION, COLLECTION, TERNARY];
    witness(source, "([Digit] 2).([Carry] 0)", &library);
}

#[test]
fn selection() {
    for (value, expected) in [("True", "2"), ("False", "0")] {
        let source = format!(
            "Invoke.Copy.Pair.([Value] {value}).Map.([Each] Filter).Reduce.([Operation] Count)"
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
                let source = format!("Invoke.Choose.{condition}.([Left] {left}).([Right] {right})");
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
                let source = format!("Invoke.{operation}.{first}.{second}");
                let target = format!(
                    "([Digit] {}).([Carry] {})",
                    digit[value % 3],
                    digit[value / 3]
                );
                check(&source, &target, &[APPLICATION, TERNARY], Outcome::Reached);
            }
            let source = format!("Invoke.Compare.([Left] {first}).([Right] {second})");
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
                "Invoke.Carry.Compose.([Left] {}).([Right] {})",
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
            let descriptor = format!("([First] Function.{first}).([Second] Function.{second})");
            let source = format!("Invoke.Compose.{descriptor}.True");
            let expected = if (first == "Not") == (second == "Not") {
                "True"
            } else {
                "False"
            };
            check(
                &source,
                &format!("{expected}.{descriptor}"),
                &[APPLICATION, BOOLEAN, COMPOSITION],
                Outcome::Reached,
            );
        }
    }
}

#[test]
fn function() {
    check(
        "Invoke.True.([Function.True] Return.False)",
        "False.([Function.True] Return.False)",
        &[APPLICATION],
        Outcome::Reached,
    );
    check(
        "Invoke.True.([Function.True] Return.False)",
        "True.([Function.True] Return.False)",
        &[APPLICATION],
        Outcome::Unreachable,
    );
}

#[test]
fn completion() {
    check(
        "Invoke.Gather.Done.Left.True",
        "([Left] True).([Right] True)",
        &[APPLICATION, COLLECTION],
        Outcome::Unreachable,
    );
    check(
        "Invoke.Gather.Done.Left.True, Invoke.Gather.Done.Right.True",
        "([Left] True).([Right] True)",
        &[APPLICATION, COLLECTION],
        Outcome::Unreachable,
    );
    for left in ["True", "False"] {
        for right in ["True", "False"] {
            let source = format!(
                "Invoke.Unpack.([Left] {left}).([Right] {right}).Map.([Each] Identity).Gather"
            );
            let target = format!("([Left] {left}).([Right] {right})");
            witness(&source, &target, &[APPLICATION, COLLECTION, PAIR]);
        }
    }
    check(
        "Function.Produce.([Value] True), Function.Produce.([Value] True)",
        "True",
        &[PRODUCTION, "[Return, Return] ()"],
        Outcome::Reached,
    );
}

#[test]
fn scheduling() {
    let source = "Invoke.Not.True, Invoke.Not.False";
    let mut baseline = None;
    for worker in [1, 2, 4] {
        let executor = photonic::executor::Executor::new(worker).unwrap();
        let mut search = {
            let program = program(source, &[APPLICATION, BOOLEAN]);
            let target = photonic::source::Program {
                rule: program.rule.clone(),
                ..parse("False,True").unwrap()
            };
            Search::new(program, target)
        };
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
            let descriptor = format!("([First] Function.{first}).([Second] Function.{second})");
            let source = format!("Invoke.Compose.{descriptor}.True");
            let wrong = if (first == "Not") == (second == "Not") {
                "False"
            } else {
                "True"
            };
            check(
                &source,
                &format!("{wrong}.{descriptor}"),
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
        "Invoke.Repeat.Empty",
        "()",
        &[APPLICATION, PRODUCTION],
        Outcome::Reached,
    );
    check(
        "Invoke.Gather.Empty",
        "()",
        &[APPLICATION, COLLECTION],
        Outcome::Reached,
    );
    check(
        "Invoke.Reduce.Empty.([Operation] And)",
        "True",
        &[APPLICATION, COLLECTION],
        Outcome::Reached,
    );
    check(
        "Invoke.Reduce.Empty.([Operation] Or)",
        "False",
        &[APPLICATION, COLLECTION],
        Outcome::Reached,
    );
}

#[test]
fn association() {
    let source = "Invoke.Pair.True.([Operation] And), Invoke.Pair.False.([Operation] And)";
    let library = [
        APPLICATION,
        BOOLEAN,
        COLLECTION,
        "[Function.Pair.True] (Reduce.Done.Left.True, Reduce.Done.Right.True) [Function.Pair.False] (Reduce.Done.Left.False, Reduce.Done.Right.False)",
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
                let source = format!("Invoke.{operation}.{}.{}", value[left], value[right]);
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
                let source = format!("Invoke.{operation}.{first}.{second}");
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
        let source = format!("Invoke.Successor.{first}");
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
            let source = format!("Invoke.Carry.Evaluate.{state}.{input}");
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
    for library in [&[][..], &[APPLICATION][..], &[position][..]] {
        check(
            "Invoke.Pack.([Position] 0).([Value] 2)",
            "([0] 2)",
            library,
            Outcome::Unreachable,
        );
    }
    check(
        "Pack(Position.0, Value.2)",
        "([0] 2)",
        &[APPLICATION, position],
        Outcome::Unreachable,
    );
    for source in [
        "Invoke.Pack.([Position] 4).([Value] 2)",
        "Invoke.Pack.([Position] 0).([Value] 3)",
        "Invoke.Pack.([Value] 2)",
    ] {
        check(
            source,
            "([0] 2)",
            &[APPLICATION, position],
            Outcome::Unreachable,
        );
    }
    check(
        "Invoke.Pack.([Position] 0).([Value] 2).Extra",
        "([0] 2).Extra",
        &[APPLICATION, position],
        Outcome::Reached,
    );
    for index in 0..4 {
        for value in 0..3 {
            let source = format!("Invoke.Unpack.([Position] {index}).([{index}] {value})");
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
            for expected in 0..3 {
                check(
                    &format!("Invoke.Pack.([Position] {index}).([Value] {value})"),
                    &format!("([{index}] {expected})"),
                    &[APPLICATION, position],
                    if expected == value {
                        Outcome::Reached
                    } else {
                        Outcome::Unreachable
                    },
                );
            }
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
        "Invoke.Unpack.([Position] 0).([1] 2)",
        "([Value] 2)",
        &[APPLICATION, position],
        Outcome::Unreachable,
    );
}

#[test]
fn generic() {
    for (input, output) in [
        ("Seed", "Flower"),
        ("7.7", "8.([Branch] Leaf)"),
        ("([Record] Item)", "([Result] ([Nested] Value))"),
    ] {
        let first = format!("([First.{input}] Return.Middle)");
        let second = format!("([Second.Middle] Return.{output})");
        let source = format!("Invoke.Compose.{input}.{first}.{second}");
        let target = format!("{output}.{first}.{second}");
        check(
            &source,
            &target,
            &[APPLICATION, COMPOSITION],
            Outcome::Reached,
        );
        check(
            &source,
            &format!("Wrong.{first}.{second}"),
            &[APPLICATION, COMPOSITION],
            Outcome::Unreachable,
        );
    }
}

#[test]
fn conflict() {
    let first = "([First.Seed] Return.Middle).([Second.Middle] Return.Flower)";
    let second = "([First.Seed] Return.Other).([Second.Other] Return.Tree)";
    let source = format!("Left.Invoke.Compose.Seed.{first}, Right.Invoke.Compose.Seed.{second}");
    check(
        &source,
        &format!("Left.Flower.{first}, Right.Tree.{second}"),
        &[APPLICATION, COMPOSITION],
        Outcome::Reached,
    );
    check(
        &source,
        &format!("Left.Tree.{first}, Right.Flower.{second}"),
        &[APPLICATION, COMPOSITION],
        Outcome::Unreachable,
    );
    check(
        &source,
        &format!("Left.Flower.{first}, Right.Flower.{second}"),
        &[APPLICATION, COMPOSITION],
        Outcome::Unreachable,
    );
}

#[test]
fn nested() {
    let source = include_str!("../../program/composition/function.wave");
    let target = include_str!("../../program/composition/function.particle");
    check(
        source,
        target,
        &[APPLICATION, COMPOSITION],
        Outcome::Reached,
    );
    for wrong in [
        target.replace("Left.Flower", "Left.Grove"),
        target.replace("Right.Grove", "Right.Flower"),
    ] {
        check(
            source,
            &wrong,
            &[APPLICATION, COMPOSITION],
            Outcome::Unreachable,
        );
    }
}

#[test]
fn extension() {
    let implementation = "[Decorate.([Function] Decorate).Envelope] Return.([Result] 11)";
    check(
        "Invoke.([Function] Decorate).Envelope",
        "([Result] 11)",
        &[APPLICATION, implementation],
        Outcome::Reached,
    );
    check(
        "Invoke.([Function] Decorate).Envelope",
        "Envelope",
        &[APPLICATION, implementation],
        Outcome::Unreachable,
    );
    check(
        "Invoke.([Function] Missing).Envelope",
        "([Result] 11)",
        &[APPLICATION],
        Outcome::Unreachable,
    );
    let implementation = "[Invert.([Each] Invert).True] Return.False [Invert.([Each] Invert).False] Return.True [Agree.([Operation] Agree).True.True] Return.True [Agree.([Operation] Agree).True.False] Return.False [Agree.([Operation] Agree).False.False] Return.False";
    witness(
        "Invoke.Copy.Pair.([Value] False).Map.([Each] Invert).Reduce.([Operation] Agree)",
        "True",
        &[APPLICATION, PRODUCTION, COLLECTION, implementation],
    );
}

#[test]
fn invocation() {
    let definition = "([Function.Seed] Return.Flower)";
    let source = format!("Invoke.Seed.{definition}");
    let target = format!("Flower.{definition}");
    check(&source, &target, &[APPLICATION], Outcome::Reached);
    check(
        &source,
        &format!("Seed.{definition}"),
        &[APPLICATION],
        Outcome::Unreachable,
    );
    let mut search = {
        let program = program(&source, &[APPLICATION]);
        let target = photonic::source::Program {
            rule: program.rule.clone(),
            ..parse(&target).unwrap()
        };
        photonic::path::Search::new(program, target)
    };
    search.run(
        100_000,
        Limit {
            state: 128,
            record: 100_000,
            cell: 128,
            world: 16,
            frame: 16,
        },
    );
    let report = search.report();
    assert_eq!(report.outcome, Outcome::Reached);
    assert_eq!(report.event.len(), 3);
}

#[test]
fn incomplete() {
    let definition = "([Function.Seed] Flower)";
    check(
        &format!("Invoke.Seed.{definition}"),
        &format!("Flower.{definition}"),
        &[APPLICATION],
        Outcome::Unreachable,
    );
    let first = "([First.Seed] Return.Middle)";
    check(
        &format!("Invoke.Compose.Seed.{first}"),
        &format!("Middle.{first}"),
        &[APPLICATION, COMPOSITION],
        Outcome::Unreachable,
    );
    let first = "([First.Seed] Middle)";
    let second = "([Second.Middle] Return.Flower)";
    check(
        &format!("Invoke.Compose.Seed.{first}.{second}"),
        &format!("Flower.{first}.{second}"),
        &[APPLICATION, COMPOSITION],
        Outcome::Unreachable,
    );
}

#[test]
fn continuation() {
    let definition = "([Function.Seed] Return.Flower)";
    let source = format!("Left.Invoke.Seed.{definition}, Right.Invoke.Seed.{definition}");
    let target = format!("Left.Flower.{definition}, Right.Flower.{definition}");
    let mut baseline = None;
    for worker in [1, 2, 4] {
        let executor = photonic::executor::Executor::new(worker).unwrap();
        let mut search = {
            let program = program(&source, &[APPLICATION]);
            let target = photonic::source::Program {
                rule: program.rule.clone(),
                ..parse(&target).unwrap()
            };
            Search::new(program, target)
        };
        search.parallel(&executor, 1, None);
        assert!(!search.report().execution.closed);
        search.parallel(
            &executor,
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
        assert!(report.execution.closed);
        assert_eq!(report.outcome, Outcome::Reached);
        let actual = format!("{:?}", report.execution);
        if let Some(expected) = &baseline {
            assert_eq!(&actual, expected);
        }
        baseline = Some(actual);
    }
}

#[test]
fn obsolete() {
    for source in ["Apply.Not.True", "Call.Not.True"] {
        check(
            source,
            "False",
            &[APPLICATION, BOOLEAN],
            Outcome::Unreachable,
        );
    }
    check(
        "Map.Ready.Left.True.([Function] Not)",
        "Done.Left.False",
        &[APPLICATION, BOOLEAN, COLLECTION],
        Outcome::Unreachable,
    );
}
