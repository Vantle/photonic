use crate::archive::{Archive, Record};
use crate::corpus::addition;
use crate::objective::{Setting, evaluate};
use code::program::Program;

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
    archive.register(&task.name, reference.cost, Some(reference.clone()));
    assert!((archive.best(&task.name) - reference.cost).abs() < 1e-12);
    assert!(archive.offer(&task.name, reference.clone()).is_none());
    let merged = photonic::lowering::parse("[Left, Right] ()").unwrap();
    let mut vocabulary = task.vocabulary.clone();
    let (program, _) = translation::lift::program(&merged, &mut vocabulary).unwrap();
    let better = record(program, true);
    assert!(better.cost < reference.cost);
    let improvement = archive.offer(&task.name, better.clone()).unwrap();
    assert_eq!(improvement.before, Some(reference.cost));
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
    assert_eq!(archive.forget(&task.name), None);
}
