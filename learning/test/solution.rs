use crate::encoding::ATOM;
use crate::objective::{Setting, evaluate, floor, memorization};
use crate::problem::Problem;
use crate::solution::{Budget, solve};
use crate::task::{Example, Task};
use code::configuration::Configuration;
use code::observation::Observation;
use code::program::Program;
use std::time::Duration;
use translation::lift;
use translation::vocabulary::Vocabulary;

const WIDE: Budget = Budget {
    size: 16,
    time: None,
};

fn parse(text: &str, vocabulary: &mut Vocabulary) -> (Program, Configuration) {
    lift::program(&photonic::lowering::parse(text).unwrap(), vocabulary).unwrap()
}

fn task(reference: Option<&str>) -> Task {
    let mut vocabulary = Vocabulary::default();
    let reference = reference.map(|text| parse(text, &mut vocabulary).0);
    let (_, input) = parse("A", &mut vocabulary);
    let (_, output) = parse("B", &mut vocabulary);
    Task {
        name: "rename".to_owned(),
        vocabulary,
        hidden: 0,
        example: vec![Example {
            input,
            output: Observation::from(&output),
        }],
        holdout: Vec::new(),
        reference,
        goal: None,
    }
}

#[test]
fn improvement() {
    let setting = Setting::default();
    let task = task(Some("[A] (B), [C] ()"));
    let reference = task.reference.as_ref().unwrap();
    let known = evaluate(reference, &task.example, &task.vocabulary, &setting);
    assert!(known.correct);
    let solution = solve(&task, known.cost, &setting, WIDE).unwrap();
    assert!(solution.proven && solution.complete);
    assert!((solution.floor - 1.25).abs() < 1e-9);
    assert!((solution.cost - 1.6).abs() < 1e-9);
    assert!(solution.cost < known.cost - 0.2);
    assert_eq!(solution.size, Some(7));
    assert!(!solution.optimal.is_empty());
    for (program, evaluation) in &solution.optimal {
        assert_eq!(evaluation.size.total(), 7);
        let again = evaluate(program, &task.example, &task.vocabulary, &setting);
        assert!((again.cost - solution.cost).abs() < 1e-9);
    }
}

#[test]
fn blind() {
    let setting = Setting::default();
    let task = task(None);
    assert!((floor(&task.example, &task.vocabulary, &setting) - 1.25).abs() < 1e-9);
    let table = memorization(&task.example, &task.vocabulary, &setting);
    assert!((table - 1.6).abs() < 1e-9);
    let problem = Problem::new(task.clone(), &setting, ATOM).unwrap();
    assert!(problem.reference.is_none());
    assert!((problem.baseline - table).abs() < 1e-9);
    let solution = solve(&task, f64::INFINITY, &setting, WIDE).unwrap();
    assert!(solution.proven && solution.complete);
    assert!((solution.cost - 1.6).abs() < 1e-9);
    assert_eq!(solution.optimal.len(), 1);
    let tight = Budget {
        size: 6,
        time: None,
    };
    let solution = solve(&task, 1.6, &setting, tight).unwrap();
    assert!(solution.proven && !solution.complete);
    assert!(solution.cost.is_infinite());
}

#[test]
fn bounded() {
    let setting = Setting::default();
    let task = task(None);
    let small = Budget {
        size: 3,
        time: None,
    };
    let solution = solve(&task, f64::INFINITY, &setting, small).unwrap();
    assert!(!solution.proven);
    assert!(solution.cost.is_infinite());
    assert_eq!(solution.size, Some(3));
    assert!((solution.gap() - 1.45).abs() < 1e-9);
    let expired = Budget {
        size: 16,
        time: Some(Duration::ZERO),
    };
    let solution = solve(&task, f64::INFINITY, &setting, expired).unwrap();
    assert!(!solution.proven);
    assert_eq!(solution.size, None);
    assert_eq!(solution.examined, 0);
}
