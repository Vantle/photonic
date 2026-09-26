use crate::emit;
use crate::lift;
use crate::vocabulary::Vocabulary;
use code::configuration::Configuration;
use code::observation::Observation;
use code::program::Program;
use photonic::execution::{self, Bound, Exploration, Walk};
use photonic::runtime::Limit;

pub fn explore(
    program: &Program,
    input: &Configuration,
    vocabulary: &Vocabulary,
    limit: Limit,
    bound: Bound,
    mut stop: impl FnMut(&Observation) -> bool,
) -> Exploration<Observation> {
    execution::explore(
        &emit::program(program, input, vocabulary),
        limit,
        bound,
        |terminal| stop(&lift::observation(terminal, vocabulary)),
    )
    .map(|terminal| lift::observation(&terminal, vocabulary))
}

pub fn walk(
    program: &Program,
    input: &Configuration,
    vocabulary: &Vocabulary,
    limit: Limit,
    bound: Bound,
    choose: impl FnMut(usize) -> usize,
) -> Walk<Observation> {
    execution::walk(
        &emit::program(program, input, vocabulary),
        limit,
        bound,
        choose,
    )
    .map(|terminal| lift::observation(&terminal, vocabulary))
}
