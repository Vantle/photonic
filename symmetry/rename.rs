use code::atom::Atom;
use code::configuration::Configuration;
use code::output::Output;
use code::particle::Particle;
use code::program::Program;
use code::rule::Rule;
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
            .map(|output| {
                Output::new(
                    particle(output.particle(), map),
                    output
                        .body()
                        .map(|body| body.iter().map(|entry| self::rule(entry, map)).collect()),
                )
            })
            .collect(),
    )
}

pub fn program(program: &Program, map: &impl Fn(Atom) -> Atom) -> Program {
    Program::from(
        program
            .rule()
            .iter()
            .map(|entry| rule(entry, map))
            .collect::<Vec<_>>(),
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
