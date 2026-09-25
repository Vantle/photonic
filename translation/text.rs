use crate::vocabulary::Vocabulary;
use code::configuration::Configuration;
use code::program::Program;
use code::rule::Rule;

pub fn rule(rule: &Rule, vocabulary: &Vocabulary) -> String {
    photonic::text::definition(&crate::emit::definition(rule, vocabulary))
}

pub fn configuration(configuration: &Configuration, vocabulary: &Vocabulary) -> String {
    crate::emit::configuration(configuration, vocabulary)
        .iter()
        .map(|particle| photonic::text::coherence(particle))
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn program(program: &Program, vocabulary: &Vocabulary) -> String {
    program
        .rule()
        .iter()
        .map(|entry| rule(entry, vocabulary) + ",\n")
        .collect()
}
