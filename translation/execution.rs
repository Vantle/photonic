use crate::emit;
use crate::lift::{Fixed, observation};
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
        |terminal| {
            observation(terminal, &mut Fixed(vocabulary)).map_or(true, |terminal| stop(&terminal))
        },
    );
    let terminal = result
        .terminal
        .iter()
        .map(|entry| observation(entry, &mut Fixed(vocabulary)))
        .collect::<Result<Vec<_>, _>>();
    Exploration {
        overflow: result.overflow || terminal.is_err(),
        terminal: terminal.unwrap_or_default(),
        state: result.state,
        cycle: result.cycle,
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
    let terminal = result
        .terminal
        .as_ref()
        .map(|entry| observation(entry, &mut Fixed(vocabulary)))
        .transpose();
    Walk {
        overflow: result.overflow || terminal.is_err(),
        terminal: terminal.unwrap_or_default(),
        work: result.work,
        depth: result.depth,
        cycle: result.cycle,
    }
}
