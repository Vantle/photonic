use code::atom::Atom;
use code::configuration::Configuration;
use code::output::Output;
use code::particle::Particle;
use code::program::Program;
use code::rule::Rule;
use code::scope::Scope;
use code::value::Value;

pub fn value(value: &Value, map: &impl Fn(Atom) -> Atom) -> Value {
    match value {
        Value::Atom(atom) => Value::Atom(map(*atom)),
        Value::Rule(rule) => Value::Rule(Box::new(self::rule(rule, map))),
    }
}

pub fn particle(particle: &Particle, map: &impl Fn(Atom) -> Atom) -> Particle {
    Particle::from(
        particle
            .value()
            .iter()
            .map(|entry| value(entry, map))
            .collect::<Vec<_>>(),
    )
}

pub fn rule(rule: &Rule, map: &impl Fn(Atom) -> Atom) -> Rule {
    Rule::new(
        rule.input()
            .iter()
            .map(|entry| particle(entry, map))
            .collect(),
        rule.output()
            .iter()
            .map(|output| match output {
                Output::Particle(value) => Output::Particle(particle(value, map)),
                Output::Scope(value) => Output::Scope(scope(value, map)),
            })
            .collect(),
    )
}

pub fn scope(scope: &Scope, map: &impl Fn(Atom) -> Atom) -> Scope {
    scope.map(&mut |entry| particle(entry, map), &mut |entry| {
        rule(entry, map)
    })
}

pub fn program(program: &Program, map: &impl Fn(Atom) -> Atom) -> Program {
    Program::new(
        program
            .rule()
            .iter()
            .map(|entry| rule(entry, map))
            .collect(),
        program
            .scope()
            .iter()
            .map(|entry| scope(entry, map))
            .collect(),
    )
}

pub fn configuration(configuration: &Configuration, map: &impl Fn(Atom) -> Atom) -> Configuration {
    Configuration::from(
        configuration
            .coherence()
            .iter()
            .map(|entry| particle(entry, map))
            .collect::<Vec<_>>(),
    )
}
