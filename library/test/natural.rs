use crate::catalog::source;
use crate::fixture::{Fixture, Random, digit, trace, value};
use photonic::prism::Outcome;
use std::cmp::Ordering;

fn compare(left: &[u64], right: &[u64]) {
    let mut fixture = Fixture::new();
    let built = fixture.stage();
    let ready = fixture.stage();
    fixture.natural(left, fixture.start(), "Operand.Left", &built);
    fixture.natural(right, &format!("Clean.{built}"), "Operand.Right", &ready);
    fixture.rule(format!("[Clean.{ready}] Function.Natural.Compare"));
    fixture.number(left, "Return.Natural.Compare.Left", "Checked.Left");
    fixture.number(right, "Return.Natural.Compare.Right", "Checked.Right");
    let verdict = match value(left).cmp(&value(right)) {
        Ordering::Less => "Less",
        Ordering::Equal => "Equal",
        Ordering::Greater => "Greater",
    };
    let summary = trace(
        &fixture.source(),
        &format!("Checked.Left, Checked.Right, Return.Natural.Compare.{verdict}"),
        &[
            source("chain", "cell"),
            source("natural", "digit"),
            source("natural", "compare"),
        ],
    );
    assert_eq!(summary.outcome, Outcome::Reached, "{left:?} {right:?}");
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
