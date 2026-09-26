use crate::catalog::LIBRARY;
use frontend::lowering::parse;
use frontend::source::{Definition, Output, Program, Value};
use std::collections::BTreeSet;

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

fn enclosure(program: &Program) -> Vec<&Program> {
    std::iter::once(program)
        .chain(program.scope.iter().flat_map(enclosure))
        .collect()
}

fn coherence(output: &Output) -> Vec<&[Value]> {
    match output {
        Output::Particle(particle) => vec![particle],
        Output::Scope(program) => enclosure(program)
            .into_iter()
            .flat_map(|scope| scope.initial.iter().map(Vec::as_slice))
            .collect(),
    }
}

fn body(output: &Output) -> Vec<&Definition> {
    match output {
        Output::Particle(_) => Vec::new(),
        Output::Scope(program) => enclosure(program)
            .into_iter()
            .flat_map(|scope| &scope.rule)
            .collect(),
    }
}

fn nested(rule: &Definition) -> Vec<&Definition> {
    std::iter::once(rule)
        .chain(rule.output.iter().flat_map(body).flat_map(nested))
        .collect()
}

fn carried(rule: &Definition) -> Vec<&Definition> {
    std::iter::once(rule)
        .chain(
            rule.output
                .iter()
                .flat_map(|output| {
                    coherence(output)
                        .into_iter()
                        .flatten()
                        .filter_map(|value| match value {
                            Value::Rule { rule } => Some(rule.as_ref()),
                            Value::Atom(_) => None,
                        })
                        .chain(body(output))
                })
                .flat_map(carried),
        )
        .collect()
}

fn every(
    reach: fn(&Definition) -> Vec<&Definition>,
) -> Vec<(&'static str, &'static str, Definition)> {
    LIBRARY
        .iter()
        .flat_map(|entry| {
            parse(&entry.source)
                .unwrap()
                .rule
                .iter()
                .flat_map(reach)
                .cloned()
                .collect::<Vec<_>>()
                .into_iter()
                .map(|rule| (entry.package.as_str(), entry.name.as_str(), rule))
        })
        .collect()
}

const ANSWER: [&str; 10] = [
    "Built", "Clean", "Lifted", "Linked", "Return", "Seen", "Stored", "Taken", "Unlinked", "Yield",
];

// A round of the sort takes runs from a lifted stack, Sort.Pile or Sort.Heap, and the sort writes a
// stack's empty linked form, Linked.Sort.Pile.Nil or Linked.Sort.Heap.Nil, only while no round takes
// from that stack, so those labels never meet the linked stacks they fit inside.
const BOOKKEEPING: (&str, &str, &str) = ("vector", "sort", "Linked");

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
    let definition = every(nested);
    let produced = every(carried)
        .into_iter()
        .flat_map(|(package, name, rule)| {
            rule.output
                .iter()
                .flat_map(coherence)
                .map(|value| (package, name, particle(value)))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    for word in ANSWER {
        let atom = Value::Atom(word.to_owned());
        let reply = produced
            .iter()
            .filter(|(_, _, value)| value.contains(&atom))
            .collect::<Vec<_>>();
        assert!(!reply.is_empty(), "no rule answers {word}");
        for (package, name, rule) in &definition {
            for pattern in rule.input.iter().map(|value| particle(value)) {
                if pattern.is_empty() || pattern.contains(&atom) {
                    continue;
                }
                for (owner, file, value) in &reply {
                    let exempt = (*package, *name, word) == BOOKKEEPING
                        && (*owner, *file, word) == BOOKKEEPING;
                    assert!(
                        exempt || !contains(value, &pattern),
                        "{package}/{name} {pattern:?} can take the answer {value:?} of {owner}/{file}"
                    );
                }
            }
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
        for value in coherence(output).into_iter().flatten() {
            spelling(value);
        }
        for nested in body(output) {
            declaration(nested, bounded);
        }
    }
}

#[test]
fn vocabulary() {
    for entry in LIBRARY.iter() {
        let library = parse(&entry.source).unwrap();
        assert!(library.initial.is_empty() && library.scope.is_empty());
        for rule in &library.rule {
            declaration(rule, BOUNDED.contains(&entry.package.as_str()));
        }
    }
}

// Map fires ([Each] X) and Reduce fires ([Operation] X) through a bare role on purpose, and Left and
// Right name both the ordered operands of scalar tables and the sides of linked operands.
const DUAL: [&str; 4] = ["Each", "Left", "Operation", "Right"];

fn field(rule: &Definition) -> Option<&str> {
    let [coherence] = rule.input.as_slice() else {
        return None;
    };
    let [Value::Atom(atom)] = coherence.as_slice() else {
        return None;
    };
    Some(atom)
}

fn gather(value: &[Value], role: &mut BTreeSet<String>, plain: &mut BTreeSet<String>) {
    for value in value {
        match value {
            Value::Atom(atom) => {
                plain.insert(atom.clone());
            }
            Value::Rule { rule } => match field(rule) {
                Some(atom) => {
                    role.insert(atom.to_owned());
                    produce(rule, role, plain);
                }
                None => walk(rule, role, plain),
            },
        }
    }
}

fn produce(rule: &Definition, role: &mut BTreeSet<String>, plain: &mut BTreeSet<String>) {
    for output in &rule.output {
        for value in coherence(output) {
            gather(value, role, plain);
        }
        for nested in body(output) {
            walk(nested, role, plain);
        }
    }
}

fn walk(rule: &Definition, role: &mut BTreeSet<String>, plain: &mut BTreeSet<String>) {
    for value in &rule.input {
        gather(value, role, plain);
    }
    produce(rule, role, plain);
}

#[test]
fn role() {
    let mut role = BTreeSet::new();
    let mut plain = BTreeSet::new();
    for entry in LIBRARY.iter() {
        for rule in &parse(&entry.source).unwrap().rule {
            walk(rule, &mut role, &mut plain);
        }
    }
    assert_eq!(
        role.intersection(&plain)
            .map(String::as_str)
            .collect::<Vec<_>>(),
        DUAL
    );
}
