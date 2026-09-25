use crate::archive::{Archive, Record};
use crate::corpus::addition;
use crate::objective::{Setting, evaluate};
use code::program::Program;

fn program(text: &str) -> Program {
    let mut vocabulary = addition(2).vocabulary;
    let (program, _) =
        translation::lift::program(&photonic::lowering::parse(text).unwrap(), &mut vocabulary)
            .unwrap();
    program
}

fn record(program: Program, general: bool) -> Record {
    let task = addition(2);
    let evaluation = evaluate(
        &program,
        &task.example,
        &task.vocabulary,
        &Setting::default(),
    );
    Record::new(program, &evaluation, general, 0)
}

#[test]
fn improvement() {
    let task = addition(2);
    let reference = record(task.reference.clone().unwrap(), true);
    let mut archive = Archive::default();
    archive.register(&task.name, reference.cost);
    assert!(archive.offer(&task.name, reference.clone()).is_some());
    assert!((archive.best(&task.name) - reference.cost).abs() < 1e-12);
    assert!(archive.offer(&task.name, reference.clone()).is_none());
    let better = record(program("[Left, Right] ()"), true);
    assert!(better.cost < reference.cost);
    let improvement = archive.offer(&task.name, better.clone()).unwrap();
    assert_eq!(improvement.before, Some(reference.cost));
    assert!((improvement.baseline - reference.cost).abs() < 1e-12);
    assert!((archive.best(&task.name) - better.cost).abs() < 1e-12);
    let mut overfit = better;
    overfit.general = false;
    overfit.cost -= 1.0;
    assert!(archive.offer(&task.name, overfit).is_none());
}

#[test]
fn partial() {
    let task = addition(2);
    let mut archive = Archive::default();
    let empty = record(Program::default(), false);
    assert!(empty.correctness < 1.0);
    assert!(archive.offer(&task.name, empty.clone()).is_none());
    assert!((archive.partial(&task.name) - empty.correctness).abs() < 1e-12);
    assert!(archive.best(&task.name).is_infinite());
}

#[test]
fn registration() {
    let task = addition(2);
    let mut archive = Archive::default();
    let known = record(program("[Left, Right] ()"), true);
    archive.register(&task.name, 3.0);
    archive.offer(&task.name, known.clone());
    let prior = archive.register(&task.name, 2.0);
    assert_eq!(prior.best, Some(known));
    assert!((prior.baseline - 3.0).abs() < 1e-12);
    let entry = archive.entry(&task.name).unwrap();
    assert!(entry.best.is_none() && entry.partial.is_none());
    assert!((entry.baseline - 2.0).abs() < 1e-12);
}
