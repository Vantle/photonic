use crate::atom::Atom;
use crate::output::Output;
use crate::particle::Particle;
use crate::rule::Rule;
use crate::value::Value;

pub const A: u16 = 0;
pub const B: u16 = 1;
pub const C: u16 = 2;
pub const D: u16 = 3;
pub const X: u16 = 5;
pub const Y: u16 = 6;

pub fn particle(atom: &[u16]) -> Particle {
    Particle::atom(&atom.iter().map(|&value| Atom(value)).collect::<Vec<_>>())
}

pub fn rule(input: &[&[u16]], output: &[&[u16]]) -> Rule {
    Rule::new(
        input.iter().map(|atom| particle(atom)).collect(),
        output
            .iter()
            .map(|atom| Output::Particle(particle(atom)))
            .collect(),
    )
}

pub fn nested(rule: Rule) -> Value {
    Value::Rule(Box::new(rule))
}
