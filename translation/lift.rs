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
    let mut output = Vec::new();
    for entry in &definition.output {
        match entry {
            source::Output::Particle(particle) => {
                output.push(Output::Particle(group(particle, naming)?));
            }
            source::Output::Scope(program) => output.extend(enclose(program, naming)?),
        }
    }
    Ok(Rule::new(input, output))
}

fn enclose<Name: Naming>(
    program: &source::Program,
    naming: &mut Name,
) -> Result<Vec<Output>, Name::Failure> {
    let rule = program
        .rule
        .iter()
        .map(|definition| lower(definition, naming))
        .collect::<Result<Vec<_>, _>>()?;
    let mut member = program
        .initial
        .iter()
        .map(|entry| group(entry, naming).map(Output::Particle))
        .collect::<Result<Vec<_>, _>>()?;
    for program in &program.scope {
        member.extend(enclose(program, naming)?);
    }
    Ok(Output::group(member, rule))
}

pub fn particle(entry: &[source::Value], vocabulary: &mut Vocabulary) -> Result<Particle, Failure> {
    group(entry, vocabulary)
}

pub fn rule(definition: &Definition, vocabulary: &mut Vocabulary) -> Result<Rule, Failure> {
    lower(definition, vocabulary)
}

pub fn scope(
    program: &source::Program,
    vocabulary: &mut Vocabulary,
) -> Result<Vec<Output>, Failure> {
    enclose(program, vocabulary)
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
    let mut coherence = source
        .initial
        .iter()
        .map(|entry| group(entry, vocabulary))
        .collect::<Result<Vec<_>, _>>()?;
    let mut scope = Vec::new();
    for program in &source.scope {
        for output in enclose(program, vocabulary)? {
            match output {
                Output::Particle(particle) => coherence.push(particle),
                Output::Scope(value) => scope.push(value),
            }
        }
    }
    Ok((Program::new(rule, scope), Configuration::from(coherence)))
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
