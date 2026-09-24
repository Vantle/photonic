use crate::vocabulary::Vocabulary;
use code::configuration::Configuration;
use code::particle::Particle;
use code::program::Program;
use code::rule::Rule;
use code::value::Value;
use photonic::source::{self, Definition, Output};

fn value(value: &Value, vocabulary: &Vocabulary) -> source::Value {
    match value {
        Value::Atom(atom) => source::Value::Atom(vocabulary.name(*atom).to_owned()),
        Value::Rule(rule) => source::Value::Rule {
            rule: Box::new(definition(rule, vocabulary)),
        },
    }
}

fn particle(particle: &Particle, vocabulary: &Vocabulary) -> Vec<source::Value> {
    particle
        .value()
        .iter()
        .map(|entry| value(entry, vocabulary))
        .collect()
}

pub fn definition(rule: &Rule, vocabulary: &Vocabulary) -> Definition {
    Definition {
        name: String::new(),
        input: rule
            .input()
            .iter()
            .map(|entry| particle(entry, vocabulary))
            .collect(),
        output: rule
            .output()
            .iter()
            .map(|output| Output {
                particle: particle(output.particle(), vocabulary),
                body: output.body().map(|body| {
                    body.iter()
                        .map(|rule| definition(rule, vocabulary))
                        .collect()
                }),
            })
            .collect(),
    }
}

pub fn rule(program: &Program, vocabulary: &Vocabulary) -> Vec<Definition> {
    program
        .rule()
        .iter()
        .map(|rule| definition(rule, vocabulary))
        .collect()
}

pub fn configuration(
    configuration: &Configuration,
    vocabulary: &Vocabulary,
) -> Vec<Vec<source::Value>> {
    configuration
        .coherence()
        .iter()
        .map(|entry| particle(entry, vocabulary))
        .collect()
}

pub fn program(
    program: &Program,
    configuration: &Configuration,
    vocabulary: &Vocabulary,
) -> source::Program {
    source::Program {
        initial: self::configuration(configuration, vocabulary),
        rule: rule(program, vocabulary),
    }
}
