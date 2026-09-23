use crate::catalog::source;
use crate::fixture::{Fixture, Random, Value, digit, reach, value};

fn library() -> [&'static str; 10] {
    [
        source("chain", "cell"),
        source("chain", "erase"),
        source("chain", "reverse"),
        source("natural", "digit"),
        source("natural", "compare"),
        source("vector", "node"),
        source("vector", "reverse"),
        source("vector", "erase"),
        source("vector", "merge"),
        source("vector", "sort"),
    ]
}

fn apply(item: &[Value], request: &str, answer: &str, expected: &[Value]) {
    let mut fixture = Fixture::new();
    let made = fixture.stage();
    let built = fixture.stage();
    let check = fixture.stage();
    fixture.vector(item, fixture.start(), &made, &built);
    fixture.rule(format!("[Clean.{built}, {made}] {request}"));
    fixture.rule(format!("[{answer}] {check}"));
    fixture.inspect(&Value::Vector(expected.to_vec()), &check, "Done");
    reach(
        &fixture.source(),
        "Done",
        &library(),
        format_args!("{request} {item:?}"),
    );
}

fn sort(item: &[Vec<u64>]) {
    let mut expected = item.to_vec();
    expected.sort_by_key(|digit| value(digit));
    apply(
        &natural(item),
        "Function.Vector.Sort",
        "Return.Vector.Sort",
        &natural(&expected),
    );
}

fn natural(digit: &[Vec<u64>]) -> Vec<Value> {
    digit
        .iter()
        .map(|digit| Value::Natural(digit.clone()))
        .collect()
}

fn numeral(value: &[u64]) -> Vec<Vec<u64>> {
    value.iter().map(|&value| digit(value)).collect()
}

fn permutation(item: &[u64]) -> Vec<Vec<u64>> {
    if item.len() <= 1 {
        return vec![item.to_vec()];
    }
    let mut result = Vec::new();
    for index in 0..item.len() {
        let mut rest = item.to_vec();
        let first = rest.remove(index);
        for mut tail in permutation(&rest) {
            tail.insert(0, first);
            if !result.contains(&tail) {
                result.push(tail);
            }
        }
    }
    result
}

fn nest() -> Vec<Vec<Value>> {
    let number = Value::number;
    let vector = Value::Vector;
    vec![
        vec![],
        vec![vector(vec![])],
        vec![vector(vec![number(1), number(2)]), vector(vec![number(3)])],
        vec![
            number(7),
            vector(vec![number(8), vector(vec![])]),
            vector(vec![vector(vec![vector(vec![number(9)])])]),
        ],
        vec![vector(vec![
            number(1),
            vector(vec![number(2), vector(vec![number(3)])]),
        ])],
    ]
}

#[test]
fn reverse() {
    for item in [0, 1, 5]
        .map(|length| natural(&numeral(&[4, 0, 7, 13, 2][..length])))
        .into_iter()
        .chain(nest())
    {
        let expected = item.iter().rev().cloned().collect::<Vec<_>>();
        apply(
            &item,
            "Function.Vector.Reverse",
            "Return.Vector.Reverse",
            &expected,
        );
    }
}

#[test]
fn erase() {
    for item in [vec![], vec![0], vec![4, 0, 7, 13, 2]]
        .map(|number| natural(&numeral(&number)))
        .into_iter()
        .chain(nest())
    {
        let mut fixture = Fixture::new();
        let made = fixture.stage();
        let built = fixture.stage();
        fixture.vector(&item, fixture.start(), &made, &built);
        fixture.rule(format!("[Clean.{built}, {made}] Function.Vector.Erase"));
        reach(
            &fixture.source(),
            "Return.Vector.Erase",
            &library(),
            format_args!("{item:?}"),
        );
    }
}

#[test]
fn order() {
    for item in permutation(&[0, 1, 2, 3]) {
        sort(&numeral(&item));
    }
    for item in permutation(&[1, 1, 2, 2]) {
        sort(&numeral(&item));
    }
    for item in [
        vec![],
        vec![7],
        (0..12).collect(),
        (0..12).rev().collect(),
        vec![7; 9],
        vec![0, 26, 1, 25, 2, 24, 3, 23],
    ] {
        sort(&numeral(&item));
    }
}

#[test]
fn random() {
    let mut random = Random::new(0x0dd5_eed5_0f5e_7a11);
    for length in 5..13 {
        for bound in [3, 10, 30, 250] {
            let item = (0..length).map(|_| random.below(bound)).collect::<Vec<_>>();
            sort(&numeral(&item));
        }
    }
}

#[test]
fn stability() {
    let pool = [
        vec![],
        vec![0],
        vec![1],
        vec![1, 0],
        vec![2],
        vec![2, 0],
        vec![0, 1],
        vec![0, 1, 0],
        vec![1, 1],
    ];
    let mut random = Random::new(0x57ab_1e00_0000_0003);
    for length in 2..10 {
        for _ in 0..3 {
            let item = (0..length)
                .map(|_| pool[random.below(pool.len() as u64) as usize].clone())
                .collect::<Vec<_>>();
            sort(&item);
        }
    }
}
