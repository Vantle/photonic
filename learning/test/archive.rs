use crate::archive::{Archive, Coverage, Proof, Record};
use crate::corpus::addition;
use crate::objective::{Setting, evaluate};
use code::program::Program;

fn program(text: &str) -> Program {
    let mut vocabulary = addition(2).vocabulary;
    let (program, _) =
        translation::lift::program(&frontend::lowering::parse(text).unwrap(), &mut vocabulary)
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

fn cost(archive: &Archive, task: &str) -> f64 {
    archive
        .best(task)
        .map_or(f64::INFINITY, |record| record.cost)
}

#[test]
fn improvement() {
    let task = addition(2);
    let reference = record(task.reference.clone().unwrap(), true);
    let mut archive = Archive::default();
    archive.register(&task.name, reference.cost);
    assert!(archive.offer(&task.name, reference.clone()).is_some());
    assert!((cost(&archive, &task.name) - reference.cost).abs() < 1e-12);
    assert!(archive.offer(&task.name, reference.clone()).is_none());
    let better = record(program("[Left, Right] ()"), true);
    assert!(better.cost < reference.cost);
    let improvement = archive.offer(&task.name, better.clone()).unwrap();
    assert_eq!(improvement.before, Some(reference.cost));
    assert!((improvement.baseline - reference.cost).abs() < 1e-12);
    assert!((cost(&archive, &task.name) - better.cost).abs() < 1e-12);
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
    assert_eq!(archive.partial(&task.name), Some(&empty));
    assert!(archive.best(&task.name).is_none());
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

#[test]
fn coverage() {
    let task = addition(2);
    let goal = Setting::default().goal;
    let merged = record(program("[Left, Right] ()"), true);
    let proven = |coverage| merged.clone().prove(Proof { goal, coverage });
    let mut archive = Archive::default();
    archive.offer(&task.name, proven(Coverage::Flat));
    assert!(archive.offer(&task.name, proven(Coverage::Whole)).is_none());
    assert_eq!(
        archive.best(&task.name).and_then(|record| record.proof),
        Some(Proof {
            goal,
            coverage: Coverage::Whole
        })
    );
    archive.offer(&task.name, proven(Coverage::Flat));
    assert_eq!(
        archive
            .best(&task.name)
            .and_then(|record| record.proof)
            .map(|proof| proof.coverage),
        Some(Coverage::Whole)
    );
}
