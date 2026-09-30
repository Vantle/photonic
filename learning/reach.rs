use code::configuration::Configuration;
use code::output::Output;
use code::particle::Particle;
use code::program::Program;
use code::rule::Rule;
use code::scope::Scope;
use code::value::Value;

pub(crate) fn particle(particle: &Particle) -> usize {
    particle
        .value()
        .iter()
        .map(|value| match value {
            Value::Atom(atom) => atom.index() + 1,
            Value::Rule(nested) => rule(nested),
        })
        .max()
        .unwrap_or(0)
}

pub(crate) fn rule(rule: &Rule) -> usize {
    rule.input()
        .iter()
        .map(particle)
        .chain(rule.output().iter().map(|output| match output {
            Output::Particle(value) => particle(value),
            Output::Scope(value) => scope(value),
        }))
        .max()
        .unwrap_or(0)
}

pub(crate) fn scope(scope: &Scope) -> usize {
    scope
        .coherence()
        .iter()
        .map(particle)
        .chain(scope.rule().iter().map(rule))
        .chain(scope.scope().iter().map(self::scope))
        .max()
        .unwrap_or(0)
}

pub(crate) fn program(program: &Program) -> usize {
    program
        .rule()
        .iter()
        .map(rule)
        .chain(program.scope().iter().map(scope))
        .max()
        .unwrap_or(0)
}

pub(crate) fn configuration(configuration: &Configuration) -> usize {
    configuration
        .coherence()
        .iter()
        .map(particle)
        .max()
        .unwrap_or(0)
}
