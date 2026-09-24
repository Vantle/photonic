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

fn member(particle: &Particle, vocabulary: &Vocabulary) -> String {
    particle
        .value()
        .iter()
        .map(|entry| value(entry, vocabulary))
        .collect::<Vec<_>>()
        .join(".")
}

fn particle(particle: &Particle, vocabulary: &Vocabulary) -> String {
    if particle.is_empty() {
        return "()".to_owned();
    }
    member(particle, vocabulary)
}

fn output(output: &Output, vocabulary: &Vocabulary) -> String {
    let explicit = member(output.particle(), vocabulary);
    let body = output
        .body()
        .unwrap_or_default()
        .iter()
        .map(|rule| self::rule(rule, vocabulary))
        .collect::<Vec<_>>();
    let content = std::iter::once(explicit)
        .chain(body)
        .filter(|entry| !entry.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    format!("({content})")
}

pub fn rule(rule: &Rule, vocabulary: &Vocabulary) -> String {
    let input = rule
        .input()
        .iter()
        .map(|entry| particle(entry, vocabulary))
        .collect::<Vec<_>>()
        .join(", ");
    if rule.output().is_empty() {
        return format!("[{input}],");
    }
    format!(
        "[{input}] {}",
        rule.output()
            .iter()
            .map(|entry| output(entry, vocabulary))
            .collect::<Vec<_>>()
            .join(" ")
    )
}

pub fn configuration(configuration: &Configuration, vocabulary: &Vocabulary) -> String {
    configuration
        .coherence()
        .iter()
        .map(|entry| particle(entry, vocabulary))
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn program(program: &Program, vocabulary: &Vocabulary) -> String {
    program
        .rule()
        .iter()
        .map(|entry| rule(entry, vocabulary) + "\n")
        .collect()
}
