use crate::failure::Failure;
use crate::vocabulary::Vocabulary;
use code::atom::Atom;
use code::configuration::Configuration;
use code::observation::{Observation, Occurrence};
use code::output::Output;
use code::particle::Particle;
use code::program::Program;
use code::rule::Rule;
use code::value::Value;
use frontend::source::{self, Definition};
use photonic::execution;
use std::convert::Infallible;

trait Naming {
    type Failure;

    fn atom(&mut self, name: &str) -> Result<Atom, Self::Failure>;
}

impl Naming for Vocabulary {
    type Failure = Failure;

    fn atom(&mut self, name: &str) -> Result<Atom, Failure> {
        self.intern(name)
    }
}

struct Fixed<'vocabulary>(&'vocabulary Vocabulary);

impl Naming for Fixed<'_> {
    type Failure = Infallible;

    fn atom(&mut self, name: &str) -> Result<Atom, Infallible> {
        Ok(self
            .0
            .find(name)
            .expect("the runtime names only atoms of the program it was given"))
    }
}

fn value<Name: Naming>(value: &source::Value, naming: &mut Name) -> Result<Value, Name::Failure> {
    match value {
        source::Value::Atom(name) => naming.atom(name).map(Value::Atom),
        source::Value::Rule { rule } => Ok(Value::Rule(Box::new(lower(rule, naming)?))),
    }
}

fn group<Name: Naming>(
    entry: &[source::Value],
    naming: &mut Name,
) -> Result<Particle, Name::Failure> {
    entry
        .iter()
        .map(|entry| value(entry, naming))
        .collect::<Result<Vec<_>, _>>()
        .map(Particle::from)
}

fn lower<Name: Naming>(definition: &Definition, naming: &mut Name) -> Result<Rule, Name::Failure> {
    let input = definition
        .input
        .iter()
        .map(|entry| group(entry, naming))
        .collect::<Result<Vec<_>, _>>()?;
    let output = definition
        .output
        .iter()
        .map(|output| {
            let body = output
                .body
                .as_ref()
                .map(|body| {
                    body.iter()
                        .map(|definition| lower(definition, naming))
                        .collect::<Result<Vec<_>, _>>()
                })
                .transpose()?;
            Ok(Output::new(group(&output.particle, naming)?, body))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Rule::new(input, output))
}

pub fn particle(entry: &[source::Value], vocabulary: &mut Vocabulary) -> Result<Particle, Failure> {
    group(entry, vocabulary)
}

pub fn rule(definition: &Definition, vocabulary: &mut Vocabulary) -> Result<Rule, Failure> {
    lower(definition, vocabulary)
}

pub fn program(
    source: &source::Program,
    vocabulary: &mut Vocabulary,
) -> Result<(Program, Configuration), Failure> {
    let rule = source
        .rule
        .iter()
        .map(|definition| lower(definition, vocabulary))
        .collect::<Result<Vec<_>, _>>()?;
    let coherence = source
        .initial
        .iter()
        .map(|entry| group(entry, vocabulary))
        .collect::<Result<Vec<_>, _>>()?;
    Ok((Program::from(rule), Configuration::from(coherence)))
}

pub(crate) fn observation(
    observation: &execution::Observation,
    vocabulary: &Vocabulary,
) -> Observation {
    let naming = &mut Fixed(vocabulary);
    Observation::new(
        observation
            .coherence
            .iter()
            .map(|entry| {
                entry
                    .iter()
                    .map(|occurrence| {
                        let Ok(value) = value(&occurrence.value, naming);
                        Occurrence {
                            id: occurrence.id as u32,
                            value,
                        }
                    })
                    .collect()
            })
            .collect(),
    )
}
