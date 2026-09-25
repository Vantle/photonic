use crate::archive::Archive;
use crate::objective::{Evaluation, TOLERANCE};
use crate::problem::Problem;
use code::program::Program;
use std::collections::HashSet;
use std::sync::Arc;

pub struct Bar {
    best: Vec<f64>,
    partial: Vec<f64>,
    overfit: HashSet<(usize, Arc<Program>)>,
    capacity: usize,
}

impl Bar {
    pub fn new(count: usize, capacity: usize) -> Self {
        Self {
            best: vec![f64::INFINITY; count],
            partial: vec![0.0; count],
            overfit: HashSet::new(),
            capacity,
        }
    }

    pub fn refresh(&mut self, archive: &Archive, problem: &[Problem]) {
        for (index, problem) in problem.iter().enumerate() {
            self.best[index] = archive.best(&problem.task.name);
            self.partial[index] = archive.partial(&problem.task.name);
        }
    }

    pub fn raise(&mut self, index: usize, program: &Arc<Program>, evaluation: &Evaluation) -> bool {
        if !evaluation.correct {
            let better = evaluation.correctness > self.partial[index] + TOLERANCE;
            if better {
                self.partial[index] = evaluation.correctness;
            }
            return better;
        }
        let better = evaluation.cost < self.best[index] - TOLERANCE
            && !self.overfit.contains(&(index, program.clone()));
        if better {
            self.best[index] = evaluation.cost;
        }
        better
    }

    pub fn reject(&mut self, index: usize, program: Arc<Program>) {
        if self.overfit.len() >= self.capacity {
            self.overfit.clear();
        }
        self.overfit.insert((index, program));
    }
}
