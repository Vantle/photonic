use crate::objective::{Evaluation, Setting, evaluate, memorization};
use crate::task::{Goal, Task};
use code::tree::walk;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum Failure {
    #[error("the reference program of {task} is incorrect on its own examples")]
    Reference { task: String },
    #[error("{task} names {count} atoms; the encoding admits {limit}")]
    Vocabulary {
        task: String,
        count: usize,
        limit: usize,
    },
    #[error("{task} has no examples")]
    Empty { task: String },
}

#[derive(Clone, Debug)]
pub struct Problem {
    pub task: Task,
    pub reference: Option<Evaluation>,
    pub baseline: f64,
    pub nesting: usize,
}

impl Problem {
    pub fn new(mut task: Task, setting: &Setting, limit: usize) -> Result<Self, Failure> {
        task.goal = Some(task.goal.unwrap_or(Goal {
            processor: setting.processor,
            size: setting.size,
        }));
        let setting = &setting.aim(task.goal);
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
            return Ok(Self {
                baseline: memorization(&task.example, &task.vocabulary, setting).max(f64::EPSILON),
                nesting: 0,
                task,
                reference: None,
            });
        };
        let reference = evaluate(program, &task.example, &task.vocabulary, setting);
        let holdout = evaluate(program, &task.holdout, &task.vocabulary, setting);
        if !reference.correct || !holdout.correct {
            return Err(Failure::Reference { task: task.name });
        }
        Ok(Self {
            baseline: reference.cost.max(f64::EPSILON),
            nesting: walk(program)
                .iter()
                .map(|node| node.depth)
                .max()
                .unwrap_or(0),
            task,
            reference: Some(reference),
        })
    }
}
