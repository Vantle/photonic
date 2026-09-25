use crate::vocabulary::Vocabulary;
use code::configuration::Configuration;
use code::output::Output;
use code::particle::Particle;
use code::program::Program;
use code::rule::Rule;
use code::value::Value;

fn value(value: &Value, vocabulary: &Vocabulary) -> String {
    match value {
        Value::Atom(atom) => vocabulary.name(*atom).to_owned(),
        Value::Rule(rule) => format!("({})", self::rule(rule, vocabulary)),
    }
}

fn join(particle: &Particle, vocabulary: &Vocabulary) -> String {
    particle
        .value()
        .iter()
        .map(|entry| value(entry, vocabulary))
        .collect::<Vec<_>>()
        .join(".")
}

fn edge(particle: &Particle, vocabulary: &Vocabulary) -> String {
    match particle.value() {
        [] => "()".to_owned(),
        [Value::Rule(rule)] => self::rule(rule, vocabulary),
        _ => join(particle, vocabulary),
    }
}

fn member(particle: &Particle, vocabulary: &Vocabulary) -> String {
    match particle.value() {
        [] => "()".to_owned(),
        [Value::Rule(_)] => format!("().{}", join(particle, vocabulary)),
        _ => join(particle, vocabulary),
    }
}

fn scope(output: &Output, body: &[Rule], vocabulary: &Vocabulary) -> String {
    let content = (!output.particle().is_empty())
        .then(|| member(output.particle(), vocabulary))
        .into_iter()
        .chain(body.iter().map(|rule| self::rule(rule, vocabulary)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("({content})")
}

fn output(output: &Output, vocabulary: &Vocabulary) -> String {
    match output.body() {
        Some(body) => scope(output, body, vocabulary),
        None => member(output.particle(), vocabulary),
    }
}

pub fn rule(rule: &Rule, vocabulary: &Vocabulary) -> String {
    let input = rule
        .input()
        .iter()
        .map(|entry| edge(entry, vocabulary))
        .collect::<Vec<_>>()
        .join(", ");
    let output = match rule.output() {
        [] => return format!("[{input}]"),
        [single] => match single.body() {
            Some(body) => scope(single, body, vocabulary),
            None => member(single.particle(), vocabulary),
        },
        several => format!(
            "({})",
            several
                .iter()
                .map(|entry| self::output(entry, vocabulary))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    };
    format!("[{input}] {output}")
}

pub fn configuration(configuration: &Configuration, vocabulary: &Vocabulary) -> String {
    configuration
        .coherence()
        .iter()
        .map(|entry| member(entry, vocabulary))
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
