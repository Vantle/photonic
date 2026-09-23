use crate::catalog::LIBRARY;
use photonic::lowering::parse;
use photonic::source::{Definition, Value};

const BOUNDED: [&str; 9] = [
    "binary",
    "boolean",
    "carry",
    "collection",
    "field",
    "function",
    "selection",
    "stream",
    "ternary",
];

fn canonical(value: &Value) -> Value {
    match value {
        Value::Atom(atom) => Value::Atom(atom.clone()),
        Value::Rule { rule } => Value::Rule {
            rule: Box::new(rule.canonical()),
        },
    }
}

fn particle(value: &[Value]) -> Vec<Value> {
    let mut result = value.iter().map(canonical).collect::<Vec<_>>();
    result.sort();
    result
}

fn contains(outer: &[Value], inner: &[Value]) -> bool {
    let mut remaining = outer.to_vec();
    inner.iter().all(|value| {
        remaining
            .iter()
            .position(|candidate| candidate == value)
            .map(|index| remaining.swap_remove(index))
            .is_some()
    })
}

fn subsumes(pattern: &[Vec<Value>], input: &[Vec<Value>], used: &mut [bool]) -> bool {
    let Some((first, rest)) = pattern.split_first() else {
        return true;
    };
    (0..input.len()).any(|index| {
        if used[index] || !contains(&input[index], first) {
            return false;
        }
        used[index] = true;
        let found = subsumes(rest, input, used);
        used[index] = false;
        found
    })
}

fn root() -> Vec<(&'static str, &'static str, Vec<Vec<Value>>)> {
    LIBRARY
        .iter()
        .flat_map(|entry| {
            parse(&entry.source).unwrap().rule.into_iter().map(|rule| {
                (
                    entry.package.as_str(),
                    entry.name.as_str(),
                    rule.input.iter().map(|value| particle(value)).collect(),
                )
            })
        })
        .collect()
}

#[test]
fn boundary() {
    let rule = root();
    for (position, (package, name, pattern)) in rule.iter().enumerate() {
        for (other, (owner, file, input)) in rule.iter().enumerate() {
            if position == other {
                continue;
            }
            assert!(
                !subsumes(pattern, input, &mut vec![false; input.len()]),
                "{package}/{name} {pattern:?} can match {owner}/{file} {input:?}"
            );
        }
    }
}

fn spelling(value: &Value) {
    match value {
        Value::Atom(atom) => {
            assert!(
                atom.chars().all(|character| character.is_ascii_digit())
                    || (atom.chars().next().is_some_and(char::is_uppercase)
                        && atom.chars().skip(1).all(char::is_lowercase)),
                "{atom}"
            );
        }
        Value::Rule { rule } => declaration(rule, false),
    }
}

fn declaration(rule: &Definition, bounded: bool) {
    if bounded {
        assert!(rule.input.len() <= 2, "{}", rule.name);
        assert!(rule.output.len() <= 2, "{}", rule.name);
    }
    for value in rule.input.iter().flatten() {
        spelling(value);
    }
    for output in &rule.output {
        for value in &output.particle {
            spelling(value);
        }
        for nested in output.body.iter().flatten() {
            declaration(nested, bounded);
        }
    }
}

#[test]
fn vocabulary() {
    for entry in LIBRARY.iter() {
        let library = parse(&entry.source).unwrap();
        assert!(library.initial.is_empty());
        for rule in &library.rule {
            declaration(rule, BOUNDED.contains(&entry.package.as_str()));
        }
    }
}
