use arithmetic::{circuit, encoding, numeral};
#[path = "../argument.rs"]
mod argument;
#[path = "../search.rs"]
mod search;

use photonic::lowering::parse;
use photonic::path::Search;
use photonic::prism::Outcome;
use photonic::runtime::Limit;

fn check(width: usize, left: u64, right: u64, expected: u64) -> Outcome {
    execute(
        &circuit::multiply(2, width, left, right, circuit::Layout::Column).unwrap(),
        &encoding::unsigned(2, width * 2, expected).unwrap(),
    )
}

fn execute(source: &str, target: &str) -> Outcome {
    let mut search = {
        let program = parse(source).unwrap();
        let target = photonic::source::Program {
            rule: program.rule.clone(),
            ..parse(target).unwrap()
        };
        Search::new(program, target)
    };
    search.run(20_000_000, crate::search::LIMIT);
    search.report().outcome
}

#[test]
fn product() {
    for left in 0..4 {
        for right in 0..4 {
            assert_eq!(
                check(2, left, right, left * right),
                Outcome::Reached,
                "{left} * {right}"
            );
            assert_eq!(check(2, left, right, left * right + 1), Outcome::Unknown);
        }
    }
    assert_eq!(check(11, 1500, 123, 184501), Outcome::Unknown);
    for (left, right) in [(1500, 123), (123, 1500), (0, 2047), (2047, 1), (2047, 2047)] {
        assert_eq!(
            check(11, left, right, left * right),
            Outcome::Reached,
            "{left} * {right}"
        );
    }
}

#[test]
fn topology() {
    let first = parse(&circuit::multiply(2, 3, 0, 0, circuit::Layout::Column).unwrap()).unwrap();
    let second = parse(&circuit::multiply(2, 3, 7, 5, circuit::Layout::Column).unwrap()).unwrap();
    assert_eq!(first.rule, second.rule);
    assert_eq!(
        parse(&encoding::unsigned(2, 32 * 2, u64::MAX).unwrap())
            .unwrap()
            .initial[0]
            .len(),
        64
    );
}

#[test]
fn exhaustive() {
    for left in 0..2 {
        for right in 0..2 {
            for expected in 0..2 {
                let mut search = {
                    let program = parse(
                        &circuit::multiply(2, 1, left, right, circuit::Layout::Column).unwrap(),
                    )
                    .unwrap();
                    let target = photonic::source::Program {
                        rule: program.rule.clone(),
                        ..parse(&encoding::unsigned(2, 2, expected).unwrap()).unwrap()
                    };
                    photonic::prism::Search::new(program, target)
                };
                search.run(100_000, None);
                let report = search.report();
                assert!(report.execution.closed);
                assert_eq!(
                    report.outcome,
                    if expected == left * right {
                        Outcome::Reached
                    } else {
                        Outcome::Unreachable
                    }
                );
            }
        }
    }
}

#[test]
fn subtraction() {
    for left in 0..4 {
        for right in 0..4 {
            let source = circuit::subtract(2, 2, left, right).unwrap();
            let expected = left as i128 - right as i128;
            assert_eq!(
                execute(&source, &encoding::difference(2, 2, expected).unwrap()),
                Outcome::Reached,
                "{left} - {right}"
            );
            assert_eq!(
                execute(
                    &source,
                    &encoding::difference(2, 2, if expected == 0 { 1 } else { 0 }).unwrap()
                ),
                Outcome::Unknown
            );
        }
    }
    for (left, right) in [(1500, 123), (123, 1500), (2047, 2047), (0, 2047)] {
        assert_eq!(
            execute(
                &circuit::subtract(2, 11, left, right).unwrap(),
                &encoding::difference(2, 11, left as i128 - right as i128).unwrap()
            ),
            Outcome::Reached
        );
    }
}

#[test]
fn division() {
    for left in 0..4 {
        for right in 0..4 {
            let source = circuit::divide(2, 2, left, right).unwrap();
            let (expected, remainder) = u64::checked_div(left, right)
                .map_or((0, left), |quotient| (quotient, left % right));
            assert_eq!(
                execute(
                    &source,
                    &encoding::quotient(2, 2, expected, remainder, right == 0).unwrap()
                ),
                Outcome::Reached,
                "{left} / {right}"
            );
            assert_eq!(
                execute(
                    &source,
                    &encoding::quotient(2, 2, expected, remainder, right != 0).unwrap()
                ),
                Outcome::Unknown
            );
            assert_eq!(
                execute(
                    &source,
                    &encoding::quotient(2, 2, expected ^ 1, remainder, right == 0).unwrap()
                ),
                Outcome::Unknown
            );
            assert_eq!(
                execute(
                    &source,
                    &encoding::quotient(2, 2, expected, remainder ^ 1, right == 0).unwrap()
                ),
                Outcome::Unknown
            );
        }
    }
    for (left, right) in [(1500, 123), (123, 1500), (2047, 1), (2047, 2047), (2047, 0)] {
        let (expected, remainder) =
            u64::checked_div(left, right).map_or((0, left), |quotient| (quotient, left % right));
        assert_eq!(
            execute(
                &circuit::divide(2, 11, left, right).unwrap(),
                &encoding::quotient(2, 11, expected, remainder, right == 0).unwrap()
            ),
            Outcome::Reached,
            "{left} / {right}"
        );
    }
}

#[test]
fn structure() {
    for program in [circuit::subtract, circuit::divide] {
        assert_eq!(
            parse(&program(2, 3, 0, 0).unwrap()).unwrap().rule,
            parse(&program(2, 3, 7, 5).unwrap()).unwrap().rule
        );
    }
}

#[test]
fn borrow() {
    let source = circuit::subtract(2, 2, 3, 3).unwrap();
    assert_eq!(
        execute(&source, "([Digit.0] 0).([Digit.1] 0).([Negative] 1)"),
        Outcome::Unknown
    );
}

#[test]
fn range() {
    for index in 0..12u64 {
        let left = (index * 71 + 13) % 256;
        let right = (index * 43 + 3) % 256;
        assert_eq!(
            execute(
                &circuit::subtract(2, 8, left, right).unwrap(),
                &encoding::difference(2, 8, left as i128 - right as i128).unwrap()
            ),
            Outcome::Reached
        );
        assert_eq!(
            execute(
                &circuit::divide(2, 8, left, right).unwrap(),
                &encoding::quotient(2, 8, left / right, left % right, false).unwrap()
            ),
            Outcome::Reached
        );
    }
}

#[test]
fn notation() {
    for text in [
        "1500",
        "1_500",
        "0b101_1101_1100",
        "0t2001120",
        "0x5dc",
        "0X5DC",
        "0o2734",
        "+1500",
    ] {
        assert_eq!(numeral::unsigned(text).unwrap(), 1500, "{text}");
        assert_eq!(numeral::signed(text).unwrap(), 1500, "{text}");
    }
    assert_eq!(numeral::signed("-0x561").unwrap(), -1377);
    assert_eq!(
        numeral::unsigned("0xFFFF_FFFF_FFFF_FFFF").unwrap(),
        u64::MAX
    );
    assert_eq!(numeral::signed(&i128::MIN.to_string()).unwrap(), i128::MIN);
    for text in [
        "", "0b", "0t", "0t3", "0t_1", "0x_1", "1_", "_1", "1__0", "0b2", "0o8", "0xG", "1.5",
        "1e3", " 1", "١", "--1",
    ] {
        assert!(numeral::signed(text).is_err(), "{text}");
        assert!(numeral::unsigned(text).is_err(), "{text}");
    }
    assert!(numeral::unsigned("-1").is_err());
    assert!(numeral::unsigned("0x1_0000_0000_0000_0000").is_err());
    assert!(numeral::signed("170141183460469231731687303715884105728").is_err());
}

#[test]
fn interface() {
    use clap::Parser;
    let argument = argument::Argument::try_parse_from([
        "word",
        "--left",
        "0x5dc",
        "--right",
        "0b111_1011",
        "--expected",
        "184_500",
    ])
    .unwrap();
    assert_eq!(argument.width().unwrap(), 7);
    let (source, target) = argument.prepare().unwrap();
    assert_eq!(execute(&source, &target), Outcome::Reached);
    let negative = argument::Argument::try_parse_from([
        "word",
        "--operation",
        "subtract",
        "--left",
        "0x7b",
        "--right",
        "0x5dc",
        "--expected=-0x561",
    ])
    .unwrap();
    let (source, target) = negative.prepare().unwrap();
    assert_eq!(execute(&source, &target), Outcome::Reached);
    for extra in [
        vec!["--width", "0"],
        vec!["--width", "21"],
        vec!["--width", "1"],
        vec!["--undefined"],
        vec!["--remainder", "1"],
    ] {
        let mut command = vec!["word", "--left", "3", "--right", "2", "--expected", "6"];
        command.extend(extra);
        assert!(
            argument::Argument::try_parse_from(command)
                .unwrap()
                .prepare()
                .is_err()
        );
    }
    let zero = argument::Argument::try_parse_from([
        "word",
        "--left",
        "0",
        "--right",
        "0",
        "--expected",
        "0",
    ])
    .unwrap();
    assert_eq!(zero.width().unwrap(), 1);
    let (source, target) = zero.prepare().unwrap();
    assert_eq!(execute(&source, &target), Outcome::Reached);
}

#[test]
fn balanced() {
    for left in 0..4 {
        for right in 0..4 {
            let source = circuit::multiply(2, 2, left, right, circuit::Layout::Balanced).unwrap();
            assert_eq!(
                execute(
                    &source,
                    &encoding::unsigned(2, 2 * 2, left * right).unwrap()
                ),
                Outcome::Reached
            );
            assert_eq!(
                execute(
                    &source,
                    &encoding::unsigned(2, 2 * 2, left * right + 1).unwrap()
                ),
                Outcome::Unknown
            );
        }
    }
    assert_eq!(
        execute(
            &circuit::multiply(2, 11, 1500, 123, circuit::Layout::Balanced).unwrap(),
            &encoding::unsigned(2, 11 * 2, 184500).unwrap()
        ),
        Outcome::Reached
    );
}

#[test]
fn addition() {
    for (left, right) in [(0, 0), (3, 7), (1500, 123), (2047, 2047)] {
        assert_eq!(
            execute(
                &circuit::add(2, 11, left, right).unwrap(),
                &encoding::unsigned(2, 12, left + right).unwrap()
            ),
            Outcome::Reached
        );
        assert_eq!(
            execute(
                &circuit::add(2, 11, left, right).unwrap(),
                &encoding::unsigned(2, 12, left + right + 1).unwrap()
            ),
            Outcome::Unknown
        );
    }
}

#[test]
fn radix() {
    for radix in [2, 3] {
        let rule = arithmetic::power::rule(radix, 8).unwrap();
        for left in 0..5 {
            for right in 0..5 {
                let source = format!(
                    "Add.({},{}), [Add,Add] (), {rule}",
                    arithmetic::power::numeral(radix, left).unwrap(),
                    arithmetic::power::numeral(radix, right).unwrap()
                );
                let mut search = {
                    let program = parse(&source).unwrap();
                    let target = photonic::source::Program {
                        rule: program.rule.clone(),
                        ..parse(&arithmetic::power::numeral(radix, left + right).unwrap()).unwrap()
                    };
                    photonic::prism::Search::new(program, target)
                };
                search.run(100_000, None);
                let report = search.report();
                assert_eq!(report.outcome, Outcome::Reached);
                assert!(report.execution.closed);
                for state in report.execution.state {
                    let total: u64 = state
                        .world
                        .iter()
                        .flat_map(|world| &world.particle)
                        .filter_map(|token| token.label.strip_prefix(&format!("{radix}^")))
                        .map(|position| (radix as u64).pow(position.parse().unwrap()))
                        .sum();
                    assert_eq!(total, left + right);
                }
            }
        }
    }
}

#[test]
fn stream() {
    for digit in [
        vec![],
        vec![true],
        vec![false, true],
        vec![true, false, true],
    ] {
        let mut body = "End".to_owned();
        for value in digit.iter().rev() {
            body = format!("({}, [Next] {body})", if *value { "1" } else { "0" });
        }
        let source = format!("Read, [Read] {body}, [0] Next, [1] Next");
        let mut search = {
            let program = parse(&source).unwrap();
            let target = photonic::source::Program {
                rule: program.rule.clone(),
                ..parse("End").unwrap()
            };
            Search::new(program, target)
        };
        search.run(
            100_000,
            Limit {
                frame: 32,
                cell: 100,
                ..Limit::default()
            },
        );
        let report = search.report();
        assert_eq!(report.outcome, Outcome::Reached);
        let visited = report
            .event
            .iter()
            .filter_map(|event| match event.rule.as_str() {
                "[0] Next" => Some(false),
                "[1] Next" => Some(true),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(visited, digit);
    }
}

#[test]
fn command() {
    let path = std::env::var_os("PHOTONIC_WORD").expect("Arithmetic runfile path");
    let command = runfiles::Runfiles::create()
        .expect("Bazel runfiles")
        .rlocation_from(path, "")
        .expect("Arithmetic executable");
    let directory =
        std::path::PathBuf::from(std::env::var_os("TEST_TMPDIR").unwrap()).join("arithmetic");
    let output = std::process::Command::new(command)
        .args([
            "--operation",
            "add",
            "--left",
            "0b11",
            "--right",
            "0x7",
            "--expected",
            "10",
        ])
        .arg("--directory")
        .arg(&directory)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Claim: 3 + 7 = 10; 2-trit operands"));
    assert!(text.contains("Reached:"));
    let program = parse(&std::fs::read_to_string(directory.join("program.wave")).unwrap()).unwrap();
    let target: photonic::source::Program =
        serde_json::from_slice(&std::fs::read(directory.join("target.json")).unwrap()).unwrap();
    assert_eq!(target.rule, program.rule);
    let mut search = Search::new(program, target);
    search.run(
        100_000,
        Limit {
            cell: 4096,
            ..Limit::default()
        },
    );
    assert_eq!(search.summary().outcome, Outcome::Reached);
    assert_eq!(argument::Operation::Add.symbol(), "+");
}

mod ternary;

#[test]
fn capacity() {
    use clap::Parser;
    for left in ["3486784401", "18446744073709551615"] {
        let argument = argument::Argument::try_parse_from([
            "word",
            "--left",
            left,
            "--right",
            "1",
            "--expected",
            "0",
        ])
        .unwrap();
        assert!(matches!(argument.width(), Err(argument::Failure::Width)));
    }
    for (radix, width) in [(2u8, 32), (3, 20)] {
        let maximum = (radix as u64).pow(width as u32) - 1;
        for layout in [circuit::Layout::Column, circuit::Layout::Balanced] {
            let source = circuit::multiply(radix, width, maximum, maximum, layout).unwrap();
            assert!(parse(&source).is_ok());
        }
    }
}

mod stream;

mod boundary;
