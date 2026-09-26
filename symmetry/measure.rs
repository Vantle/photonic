use code::atom::Atom;
use code::output::Output;
use code::particle::Particle;
use code::rule::Rule;
use code::scope::Scope;
use code::value::Value;
use std::collections::BTreeSet;

fn value(value: &Value, atom: &mut BTreeSet<Atom>) -> usize {
    match value {
        Value::Atom(entry) => {
            atom.insert(*entry);
            1
        }
        Value::Rule(nested) => rule(nested, atom),
    }
}

pub(crate) fn particle(particle: &Particle, atom: &mut BTreeSet<Atom>) -> usize {
    1 + particle
        .value()
        .iter()
        .map(|entry| value(entry, atom))
        .sum::<usize>()
}

pub(crate) fn rule(rule: &Rule, atom: &mut BTreeSet<Atom>) -> usize {
    let mut size = 1;
    for entry in rule.input() {
        size += particle(entry, atom);
    }
    for output in rule.output() {
        size += match output {
            Output::Particle(value) => particle(value, atom),
            Output::Scope(value) => scope(value, atom),
        };
    }
    size
}

pub(crate) fn scope(scope: &Scope, atom: &mut BTreeSet<Atom>) -> usize {
    let mut size = 0;
    for entry in scope.coherence() {
        size += particle(entry, atom);
    }
    for entry in scope.rule() {
        size += rule(entry, atom);
    }
    for entry in scope.scope() {
        size += self::scope(entry, atom);
    }
    size
}
