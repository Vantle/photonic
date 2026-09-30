use crate::catalog::LIBRARY;
use crate::fixture::{Fixture, digit, reach};

fn library() -> Vec<&'static str> {
    LIBRARY
        .iter()
        .filter(|entry| ["chain", "integer", "natural"].contains(&entry.package.as_str()))
        .map(|entry| entry.source.as_str())
        .collect()
}

// An operand as the caller writes it: a sign word beside a numeral, so zero can be written with
// either sign.
#[derive(Clone, Copy, Debug)]
struct Operand {
    negative: bool,
    magnitude: u64,
}

impl Operand {
    fn new(value: i64) -> Self {
        Self {
            negative: value < 0,
            magnitude: value.unsigned_abs(),
        }
    }

    fn value(self) -> i64 {
        let magnitude = i64::try_from(self.magnitude).unwrap();
        if self.negative { -magnitude } else { magnitude }
    }
}

fn sign(negative: bool) -> &'static str {
    if negative { "Negative" } else { "Positive" }
}

fn check(operation: &str, left: Operand, right: Operand, answer: Option<i64>) {
    let mut fixture = Fixture::new();
    let built = fixture.stage();
    let ready = fixture.stage();
    fixture.natural(
        &digit(left.magnitude),
        fixture.start(),
        &format!("Operand.Left.{}", sign(left.negative)),
        &built,
    );
    fixture.natural(
        &digit(right.magnitude),
        &format!("Clean.{built}"),
        &format!("Operand.Right.{}", sign(right.negative)),
        &ready,
    );
    fixture.rule(format!("[Clean.{ready}] Function.Integer.{operation}"));
    let done = fixture.stage();
    match answer {
        Some(result) => fixture.number(
            &digit(result.unsigned_abs()),
            &format!("Return.Integer.{operation}.{}", sign(result < 0)),
            &done,
        ),
        None => fixture.rule(format!("[Return.Integer.{operation}.Error.Divisor] {done}")),
    }
    reach(
        &fixture.source(),
        &done,
        &library(),
        format_args!("{operation} {left:?} {right:?} => {answer:?}"),
    );
}

fn table(operation: &str, answer: impl Fn(i64, i64) -> Option<i64>) {
    for left in -4..=4 {
        for right in -4..=4 {
            check(
                operation,
                Operand::new(left),
                Operand::new(right),
                answer(left, right),
            );
        }
    }
}

#[test]
fn add() {
    table("Add", |left, right| Some(left + right));
}

#[test]
fn subtract() {
    table("Subtract", |left, right| Some(left - right));
}

#[test]
fn multiply() {
    table("Multiply", |left, right| Some(left * right));
}

#[test]
fn divide() {
    table("Divide", i64::checked_div);
}

// A negative zero operand reads as zero, and every answer of zero is Positive.
#[test]
fn result() {
    let zero = Operand {
        negative: true,
        magnitude: 0,
    };
    for other in [-2, 0, 2].map(Operand::new) {
        for (operation, answer) in [
            ("Add", Some(other.value())),
            ("Subtract", Some(-other.value())),
            ("Multiply", Some(0)),
            ("Divide", (other.value() != 0).then_some(0)),
        ] {
            check(operation, zero, other, answer);
        }
        check("Subtract", other, zero, Some(other.value()));
        check("Divide", other, zero, None);
    }
}
