use crate::objective::{Setting, behavior};
use crate::task::{Example, Task};
use code::configuration::Configuration;
use code::observation::Observation;
use code::particle::Particle;
use code::program::Program;
use thiserror::Error;
use translation::lift;
use translation::vocabulary::Vocabulary;

#[derive(Debug, Error)]
pub enum Failure {
    #[error("could not parse {origin}: {message}")]
    Parse { origin: String, message: String },
    #[error(transparent)]
    Lift(#[from] translation::failure::Failure),
    #[error("the input {origin} declares rules; inputs hold only initial coherences")]
    Input { origin: String },
    #[error("the program has no input: pass inputs or give the program initial coherences")]
    Empty,
    #[error("the program does not reach one unique result from input {origin}")]
    Behavior { origin: String },
    #[error("a task defined by tests needs at least one test")]
    Untested,
}

pub struct Source {
    pub origin: String,
    pub text: String,
}

fn parse(source: &Source) -> Result<photonic::source::Program, Failure> {
    photonic::lowering::parse(&source.text).map_err(|failure| Failure::Parse {
        origin: source.origin.clone(),
        message: failure.to_string(),
    })
}

fn configuration(source: &Source, vocabulary: &mut Vocabulary) -> Result<Configuration, Failure> {
    let (lifted, coherence) = lift::program(&parse(source)?, vocabulary)?;
    if !lifted.rule().is_empty() {
        return Err(Failure::Input {
            origin: source.origin.clone(),
        });
    }
    Ok(coherence)
}

pub fn define(name: &str, pair: &[(Source, Source)]) -> Result<Task, Failure> {
    if pair.is_empty() {
        return Err(Failure::Untested);
    }
    let mut vocabulary = Vocabulary::default();
    let example = pair
        .iter()
        .map(|(input, output)| {
            Ok(Example {
                input: configuration(input, &mut vocabulary)?,
                output: Observation::from(&configuration(output, &mut vocabulary)?),
            })
        })
        .collect::<Result<Vec<_>, Failure>>()?;
    Ok(Task {
        name: name.to_owned(),
        vocabulary,
        hidden: 0,
        example,
        holdout: Vec::new(),
        reference: None,
        goal: None,
    })
}

pub fn import(
    name: &str,
    program: &[Source],
    input: &[Source],
    setting: &Setting,
) -> Result<Task, Failure> {
    let mut vocabulary = Vocabulary::default();
    let mut rule = Vec::new();
    let mut initial: Vec<Particle> = Vec::new();
    for source in program {
        let (lifted, configuration) = lift::program(&parse(source)?, &mut vocabulary)?;
        rule.extend(Vec::from(lifted));
        initial.extend(Vec::from(configuration));
    }
    let mut given = Vec::new();
    for source in input {
        given.push((
            source.origin.clone(),
            configuration(source, &mut vocabulary)?,
        ));
    }
    if given.is_empty() {
        if initial.is_empty() {
            return Err(Failure::Empty);
        }
        given.push((
            program
                .iter()
                .map(|source| source.origin.clone())
                .collect::<Vec<_>>()
                .join(" "),
            Configuration::from(initial),
        ));
    }
    let reference = Program::from(rule);
    let example = given
        .into_iter()
        .map(|(origin, input)| {
            behavior(&reference, &input, &vocabulary, setting)
                .map(|output| Example { input, output })
                .ok_or(Failure::Behavior { origin })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Task {
        name: name.to_owned(),
        vocabulary,
        hidden: 0,
        example,
        holdout: Vec::new(),
        reference: Some(reference),
        goal: None,
    })
}
