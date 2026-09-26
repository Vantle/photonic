use crate::vocabulary::Vocabulary;
use code::configuration::Configuration;
use code::program::Program;
use code::rule::Rule;
use code::scope::Scope;

pub fn rule(rule: &Rule, vocabulary: &Vocabulary) -> String {
    frontend::text::definition(&crate::emit::definition(rule, vocabulary))
}

pub fn scope(scope: &Scope, vocabulary: &Vocabulary) -> String {
    frontend::text::scope(&crate::emit::scope(scope, vocabulary))
}

pub fn configuration(configuration: &Configuration, vocabulary: &Vocabulary) -> String {
    crate::emit::configuration(configuration, vocabulary)
        .iter()
        .map(|particle| frontend::text::coherence(particle))
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn program(program: &Program, vocabulary: &Vocabulary) -> String {
    program
        .rule()
        .iter()
        .map(|entry| rule(entry, vocabulary))
        .chain(program.scope().iter().map(|entry| scope(entry, vocabulary)))
        .map(|entry| entry + ",\n")
        .collect()
}
