use crate::catalog::LIBRARY;
use crate::fixture::{Fixture, Random, digit, reach, value};
use std::cmp::Ordering;

fn library() -> Vec<&'static str> {
    LIBRARY
        .iter()
        .filter(|entry| entry.package == "chain" || entry.package == "natural")
        .map(|entry| entry.source.as_str())
        .collect()
}

fn verify(source: String, answer: &[(&str, Option<Vec<u64>>)], fixture: &mut Fixture) {
    let checked = answer
        .iter()
        .map(|(label, digit)| {
            let checked = fixture.stage();
            match digit {
                Some(digit) => fixture.number(digit, label, &checked),
                None => fixture.rule(format!("[{label}] {checked}")),
            }
            checked
        })
        .collect::<Vec<_>>();
    reach(
        &fixture.source(),
        &checked.join(", "),
        &library(),
        format_args!("{source} => {answer:?}"),
    );
}

fn binary(operation: &str, left: &[u64], right: &[u64], answer: &[(&str, Option<Vec<u64>>)]) {
    let mut fixture = Fixture::new();
    let built = fixture.stage();
    let ready = fixture.stage();
    fixture.natural(left, fixture.start(), "Operand.Left", &built);
    fixture.natural(right, &format!("Clean.{built}"), "Operand.Right", &ready);
    fixture.rule(format!("[Clean.{ready}] Function.Natural.{operation}"));
    verify(
        format!("{operation} {left:?} {right:?}"),
        answer,
        &mut fixture,
    );
}

fn unary(operation: &str, input: &[u64], answer: &[(&str, Option<Vec<u64>>)]) {
    let mut fixture = Fixture::new();
    let built = fixture.stage();
    let hold = fixture.stage();
    fixture.natural(input, fixture.start(), &hold, &built);
    fixture.rule(format!(
        "[Clean.{built}, {hold}] Function.Natural.{operation}"
    ));
    verify(format!("{operation} {input:?}"), answer, &mut fixture);
}

fn number(label: &str, value: u64) -> (&str, Option<Vec<u64>>) {
    (label, Some(digit(value)))
}

fn arithmetic(left: &[u64], right: &[u64]) {
    let (first, second) = (value(left), value(right));
    binary(
        "Add",
        left,
        right,
        &[number("Return.Natural.Add", first + second)],
    );
    binary(
        "Multiply",
        left,
        right,
        &[number("Return.Natural.Multiply", first * second)],
    );
    binary(
        "Subtract",
        left,
        right,
        &[if first >= second {
            number("Return.Natural.Subtract.Number", first - second)
        } else {
            ("Return.Natural.Subtract.Error.Underflow", None)
        }],
    );
    binary(
        "Difference",
        left,
        right,
        &[if first >= second {
            number("Return.Natural.Difference.Positive", first - second)
        } else {
            number("Return.Natural.Difference.Negative", second - first)
        }],
    );
    binary(
        "Divide",
        left,
        right,
        &match (first.checked_div(second), first.checked_rem(second)) {
            (Some(quotient), Some(remainder)) => vec![
                number("Return.Natural.Divide.Quotient", quotient),
                number("Return.Natural.Divide.Remainder", remainder),
            ],
            _ => vec![("Return.Natural.Divide.Error.Divisor", None)],
        },
    );
}

fn single(input: &[u64]) {
    let count = value(input);
    let mut successor = digit(count + 1);
    successor.resize(successor.len().max(input.len()), 0);
    unary(
        "Successor",
        input,
        &[("Return.Natural.Successor", Some(successor))],
    );
    unary(
        "Normalize",
        input,
        &[number("Return.Natural.Normalize", count)],
    );
    let reversed = input.iter().rev().copied().collect::<Vec<_>>();
    unary(
        "Complement",
        &reversed,
        &[number(
            "Return.Natural.Complement",
            3u64.pow(u32::try_from(input.len()).unwrap()) - count,
        )],
    );
    unary(
        "Copy",
        input,
        &[
            ("Return.Natural.Copy.Left", Some(input.to_vec())),
            ("Return.Natural.Copy.Right", Some(input.to_vec())),
        ],
    );
}

fn compare(left: &[u64], right: &[u64]) {
    let verdict = match value(left).cmp(&value(right)) {
        Ordering::Less => "Less",
        Ordering::Equal => "Equal",
        Ordering::Greater => "Greater",
    };
    binary(
        "Compare",
        left,
        right,
        &[
            (format!("Return.Natural.Compare.{verdict}").as_str(), None),
            ("Return.Natural.Compare.Left", Some(left.to_vec())),
            ("Return.Natural.Compare.Right", Some(right.to_vec())),
        ],
    );
}

#[test]
fn small() {
    for left in 0..27 {
        for right in 0..27 {
            compare(&digit(left), &digit(right));
        }
    }
}

#[test]
fn padding() {
    for (left, right) in [
        (vec![0], vec![]),
        (vec![1, 0], vec![1]),
        (vec![2, 1, 0, 0], vec![2, 1]),
        (vec![0, 0, 1], vec![2, 2]),
        (vec![1, 1, 0], vec![2, 1]),
        (vec![2], vec![1, 0, 0]),
    ] {
        compare(&left, &right);
        compare(&right, &left);
        arithmetic(&left, &right);
        arithmetic(&right, &left);
    }
}

#[test]
fn wide() {
    let mut random = Random::new(0x5eed_1234_abcd_0001);
    for _ in 0..60 {
        let left = random.below(3u64.pow(12));
        let right = match random.below(3) {
            0 => left,
            1 => left + random.below(9),
            _ => random.below(3u64.pow(12)),
        };
        compare(&digit(left), &digit(right));
    }
}

#[test]
fn table() {
    for left in 0..9 {
        for right in 0..9 {
            arithmetic(&digit(left), &digit(right));
        }
    }
}

#[test]
fn span() {
    let mut random = Random::new(0x0a71_7e5e_0000_0009);
    for _ in 0..12 {
        let left = random.below(3u64.pow(6));
        let right = random.below(3u64.pow(4)) + 1;
        arithmetic(&digit(left), &digit(right));
    }
}

#[test]
fn unit() {
    for count in 0..27 {
        single(&digit(count));
    }
    for input in [
        vec![0],
        vec![0, 0],
        vec![1, 0],
        vec![2, 2, 0],
        vec![0, 1, 0, 0],
    ] {
        single(&input);
    }
}
