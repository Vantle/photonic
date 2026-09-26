use crate::objective::{Setting, evaluate, memorization};
use crate::task::Task;
use crate::tree::walk;
use code::configuration::Configuration;
use code::particle::Particle;
use code::program::Program;
use machine::exploration::fits;
use machine::flat::Flat;
use machine::state::State;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum Failure {
    #[error("the reference program of {task} is incorrect on its own examples")]
    Reference { task: String },
    #[error("the reference program of {task} is correct only beyond the training limits")]
    Limit { task: String },
    #[error(
        "the expected output of test {} of {task} holds more than the training limits admit",
        .test + 1
    )]
    Output { task: String, test: usize },
    #[error("{task} names {count} atoms; the encoding admits {limit}")]
    Vocabulary {
        task: String,
        count: usize,
        limit: usize,
    },
    #[error("{task} has no examples")]
    Empty { task: String },
    #[error("{task} has no room left in its vocabulary for hidden atoms")]
    Hidden {
        task: String,
        source: translation::failure::Failure,
    },
}

#[derive(Clone, Debug)]
pub struct Problem {
    pub task: Task,
    pub(crate) baseline: f64,
    pub(crate) nesting: usize,
}

fn bounded(configuration: &Configuration, setting: &Setting) -> bool {
    let coherence = configuration.coherence();
    let Some(state) = State::new(configuration) else {
        return coherence.len() <= setting.admission.coherence
            && coherence.iter().map(Particle::len).sum::<usize>() <= setting.admission.occurrence;
    };
    Flat::new(&Program::default()).is_some_and(|empty| fits(&empty, &state, &setting.limit))
}

impl Problem {
    pub(crate) fn new(mut task: Task, setting: &Setting, limit: usize) -> Result<Self, Failure> {
        task.goal = Some(task.goal.unwrap_or(setting.goal));
        let setting = &setting.aim(task.goal);
        let thorough = &setting.thorough();
        if task.vocabulary.len() > limit {
            return Err(Failure::Vocabulary {
                task: task.name.clone(),
                count: task.vocabulary.len(),
                limit,
            });
        }
        if task.example.is_empty() {
            return Err(Failure::Empty { task: task.name });
        }
        let Some(program) = &task.reference else {
            if let Some(test) = task
                .example
                .iter()
                .position(|example| !bounded(&example.output.configuration(), setting))
            {
                return Err(Failure::Output {
                    task: task.name,
                    test,
                });
            }
            return Ok(Self {
                baseline: memorization(&task.example, &task.vocabulary, thorough).max(f64::EPSILON),
                nesting: 0,
                task,
            });
        };
        let reference = evaluate(program, &task.example, &task.vocabulary, thorough);
        let holdout = evaluate(program, &task.holdout, &task.vocabulary, thorough);
        if !reference.correct || !holdout.correct {
            return Err(Failure::Reference { task: task.name });
        }
        if !evaluate(program, &task.example, &task.vocabulary, setting).correct {
            return Err(Failure::Limit { task: task.name });
        }
        Ok(Self {
            baseline: reference.cost.max(f64::EPSILON),
            nesting: walk(program)
                .iter()
                .map(|node| node.depth)
                .max()
                .unwrap_or(0),
            task,
        })
    }
}
