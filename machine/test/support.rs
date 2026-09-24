use crate::flat::Flat;
use crate::state::State;
use code::atom::Atom;
use code::configuration::Configuration;
use code::output::Output;
use code::particle::Particle;
use code::program::Program;
use code::rule::Rule;

pub const A: u16 = 0;
pub const B: u16 = 1;
pub const C: u16 = 2;
pub const D: u16 = 3;
pub const E: u16 = 4;
pub const X: u16 = 5;

pub fn particle(atom: &[u16]) -> Particle {
    Particle::atom(&atom.iter().map(|&value| Atom(value)).collect::<Vec<_>>())
}

pub fn configuration(coherence: &[&[u16]]) -> Configuration {
    Configuration::from(
        coherence
            .iter()
            .map(|atom| particle(atom))
            .collect::<Vec<_>>(),
    )
}

pub fn rule(input: &[&[u16]], output: &[&[u16]]) -> Rule {
    Rule::new(
        input.iter().map(|atom| particle(atom)).collect(),
        output
            .iter()
            .map(|atom| Output::plain(particle(atom)))
            .collect(),
    )
}

pub fn program(rule: Vec<Rule>) -> Flat {
    Flat::new(&Program::from(rule)).expect("test programs are flat")
}

pub fn state(coherence: &[&[u16]]) -> State {
    State::new(&configuration(coherence)).expect("test configurations are flat")
}
