use crate::emit;
use crate::lift::{self, Fixed};
use crate::vocabulary::Vocabulary;
use code::configuration::Configuration;
use code::observation::Observation;
use code::program::Program;
use photonic::execution::{self, Bound};
use photonic::runtime::Limit;

#[derive(Clone, Debug, Default)]
pub struct Exploration {
    pub terminal: Vec<Observation>,
    pub state: usize,
    pub cycle: bool,
    pub overflow: bool,
    pub truncated: bool,
}

impl Exploration {
    pub fn complete(&self) -> bool {
        !self.cycle && !self.overflow && !self.truncated
    }
}

#[derive(Clone, Debug, Default)]
pub struct Walk {
    pub terminal: Option<Observation>,
    pub work: usize,
    pub depth: usize,
    pub cycle: bool,
    pub overflow: bool,
}

fn observe(terminal: &execution::Observation, vocabulary: &Vocabulary) -> Observation {
    lift::observation(terminal, &mut Fixed(vocabulary))
        .expect("the runtime names only atoms of the program it was given")
}

pub fn explore(
    program: &Program,
    input: &Configuration,
    vocabulary: &Vocabulary,
    limit: Limit,
    bound: Bound,
    mut stop: impl FnMut(&Observation) -> bool,
) -> Exploration {
    let result = execution::explore(
        &emit::program(program, input, vocabulary),
        limit,
        bound,
        |terminal| stop(&observe(terminal, vocabulary)),
    );
    Exploration {
        terminal: result
            .terminal
            .iter()
            .map(|entry| observe(entry, vocabulary))
            .collect(),
        state: result.state,
        cycle: result.cycle,
        overflow: result.overflow,
        truncated: result.truncated,
    }
}

pub fn walk(
    program: &Program,
    input: &Configuration,
    vocabulary: &Vocabulary,
    limit: Limit,
    bound: Bound,
    choose: impl FnMut(usize) -> usize,
) -> Walk {
    let result = execution::walk(
        &emit::program(program, input, vocabulary),
        limit,
        bound,
        choose,
    );
    Walk {
        terminal: result
            .terminal
            .as_ref()
            .map(|entry| observe(entry, vocabulary)),
        work: result.work,
        depth: result.depth,
        cycle: result.cycle,
        overflow: result.overflow,
    }
}
