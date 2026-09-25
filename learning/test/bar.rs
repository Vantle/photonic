use crate::archive::Archive;
use crate::bar::Bar;
use crate::corpus::addition;
use crate::edit::seed;
use crate::encoding::ATOM;
use crate::objective::{Evaluation, Setting, Size};
use crate::problem::Problem;
use code::atom::Atom;
use code::program::Program;
use std::sync::Arc;

fn correct(cost: f64) -> Evaluation {
    Evaluation {
        outcome: Vec::new(),
        correctness: 1.0,
        correct: true,
        verified: true,
        time: cost,
        work: 0.0,
        span: 0.0,
        size: Size::default(),
        cost,
    }
}

#[test]
fn overfit() {
    let problem = [Problem::new(addition(2), &Setting::default().thorough(), ATOM).unwrap()];
    let archive = Archive::default();
    let mut bar = Bar::new(problem.len(), 8);
    bar.refresh(&archive, &problem);
    let first = Arc::new(Program::from(vec![seed(Atom(0))]));
    let second = Arc::new(Program::from(vec![seed(Atom(1))]));
    assert!(bar.raise(0, &first, &correct(2.0)));
    assert!(!bar.raise(0, &second, &correct(2.5)));
    bar.reject(0, first.clone());
    bar.refresh(&archive, &problem);
    assert!(!bar.raise(0, &first, &correct(2.0)));
    assert!(bar.raise(0, &second, &correct(2.5)));
}
