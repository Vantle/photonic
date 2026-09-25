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
use photonic::execution;
use photonic::source::{self, Definition};

pub trait Naming {
    fn atom(&mut self, name: &str) -> Result<Atom, Failure>;
}

impl Naming for Vocabulary {
    fn atom(&mut self, name: &str) -> Result<Atom, Failure> {
        self.intern(name)
    }
}

pub(crate) struct Fixed<'vocabulary>(pub &'vocabulary Vocabulary);

impl Naming for Fixed<'_> {
    fn atom(&mut self, name: &str) -> Result<Atom, Failure> {
        self.0.find(name).ok_or_else(|| Failure::Unknown {
            name: name.to_owned(),
        })
    }
}

pub fn value(value: &source::Value, naming: &mut impl Naming) -> Result<Value, Failure> {
    match value {
        source::Value::Atom(name) => naming.atom(name).map(Value::Atom),
        source::Value::Rule { rule } => Ok(Value::Rule(Box::new(self::rule(rule, naming)?))),
    }
}

fn particle(entry: &[source::Value], naming: &mut impl Naming) -> Result<Particle, Failure> {
    entry
        .iter()
        .map(|entry| value(entry, naming))
        .collect::<Result<Vec<_>, _>>()
        .map(Particle::from)
}

pub fn rule(definition: &Definition, naming: &mut impl Naming) -> Result<Rule, Failure> {
    let input = definition
        .input
        .iter()
        .map(|entry| particle(entry, naming))
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
                        .map(|definition| rule(definition, naming))
                        .collect::<Result<Vec<_>, _>>()
                })
                .transpose()?;
            Ok(Output::new(particle(&output.particle, naming)?, body))
        })
        .collect::<Result<Vec<_>, Failure>>()?;
    Ok(Rule::new(input, output))
}

pub fn program(
    source: &source::Program,
    naming: &mut impl Naming,
) -> Result<(Program, Configuration), Failure> {
    let rule = source
        .rule
        .iter()
        .map(|definition| rule(definition, naming))
        .collect::<Result<Vec<_>, _>>()?;
    let coherence = source
        .initial
        .iter()
        .map(|entry| particle(entry, naming))
        .collect::<Result<Vec<_>, _>>()?;
    Ok((Program::from(rule), Configuration::from(coherence)))
}

pub(crate) fn observation(
    observation: &execution::Observation,
    naming: &mut impl Naming,
) -> Result<Observation, Failure> {
    observation
        .coherence
        .iter()
        .map(|entry| {
            entry
                .iter()
                .map(|occurrence| {
                    Ok(Occurrence {
                        id: occurrence.id as u32,
                        value: value(&occurrence.value, naming)?,
                    })
                })
                .collect::<Result<Vec<_>, Failure>>()
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Observation::new)
}
