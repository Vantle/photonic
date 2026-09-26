use crate::archive::{Archive, Coverage, Proof, Record};
use crate::corpus::addition;
use crate::edit::seed;
use crate::encoding::ATOM;
use crate::objective::{Setting, TOLERANCE, evaluate};
use crate::problem::Problem;
use crate::renewal::renew;
use crate::task::{Example, Goal, Task};
use code::atom::Atom;
use code::observation::Observation;
use code::program::Program;
use translation::lift;
use translation::vocabulary::Vocabulary;

fn program(text: &str, vocabulary: &mut Vocabulary) -> Program {
    lift::program(&frontend::lowering::parse(text).unwrap(), vocabulary)
        .unwrap()
        .0
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

fn task(name: &str, rule: &str, input: &str, output: &str) -> (Task, Program) {
    let mut vocabulary = Vocabulary::default();
    for atom in ["A", "D", "B", "C", "E"] {
        vocabulary.intern(atom).unwrap();
    }
    let program = program(rule, &mut vocabulary);
    let input = lift::program(&frontend::lowering::parse(input).unwrap(), &mut vocabulary)
        .unwrap()
        .1;
    let output = lift::program(&frontend::lowering::parse(output).unwrap(), &mut vocabulary)
        .unwrap()
        .1;
    let task = Task {
        name: name.to_owned(),
        vocabulary,
        example: vec![Example {
            input,
            output: Observation::from(&output),
        }],
        holdout: Vec::new(),
        reference: None,
        goal: None,
    };
    (task, program)
}

fn best(archive: &Archive, task: &Task) -> Option<Record> {
    archive.best(&task.name).cloned()
}

fn whole(goal: Goal) -> Proof {
    Proof {
        goal,
        coverage: Coverage::Whole,
    }
}

#[test]
fn proof() {
    let setting = Setting::default();
    let task = addition(2);
    let merged = program("[Left, Right] ()", &mut task.vocabulary.clone());
    let mut archive = Archive::default();
    assert!(
        archive
            .offer(&task.name, record(merged.clone(), true))
            .is_some()
    );
    assert!(
        archive
            .offer(
                &task.name,
                record(merged.clone(), true).prove(whole(setting.goal))
            )
            .is_none()
    );
    let proven = |archive: &Archive| {
        best(archive, &task)
            .filter(|record| record.program == merged)
            .map(|record| record.proof)
    };
    assert_eq!(proven(&archive), Some(Some(whole(setting.goal))));
    let problem = Problem::new(task.clone(), &setting, ATOM).unwrap();
    renew(&mut archive, &problem, &setting);
    assert_eq!(proven(&archive), Some(Some(whole(setting.goal))));
    let aimed = Task {
        goal: Some(Goal::new(1.0, setting.goal.size()).unwrap()),
        ..task.clone()
    };
    let problem = Problem::new(aimed, &setting, ATOM).unwrap();
    renew(&mut archive, &problem, &setting);
    assert_eq!(proven(&archive), Some(None));
}

#[test]
fn redefinition() {
    let setting = Setting::default();
    let task = addition(2);
    let foreign = Program::from(vec![seed(Atom(task.vocabulary.len() as u16))]);
    let stale = Record {
        correctness: 1.0,
        cost: 0.1,
        ..record(program("[Left] ()", &mut task.vocabulary.clone()), true)
    };
    let mut archive = Archive::default();
    archive.offer(&task.name, record(foreign.clone(), false));
    archive.offer(&task.name, stale.clone());
    assert_eq!(best(&archive, &task), Some(stale));
    let problem = Problem::new(task.clone(), &setting, ATOM).unwrap();
    renew(&mut archive, &problem, &setting);
    let entry = archive.entry(&task.name).unwrap();
    assert_eq!(
        entry.best.as_ref().map(|record| &record.program),
        task.reference.as_ref()
    );
    assert!(
        entry
            .partial
            .as_ref()
            .is_none_or(|record| record.program != foreign)
    );
}

#[test]
fn cost() {
    let setting = Setting {
        budget: 1,
        ..Setting::default()
    };
    let (task, program) = task("fork", "[A] D, [D, B] C, [B] E, [D, E] C", "A, B", "C");
    let searched = evaluate(&program, &task.example, &task.vocabulary, &setting);
    let verified = evaluate(
        &program,
        &task.example,
        &task.vocabulary,
        &setting.thorough(),
    );
    assert!(searched.correct && !searched.verified && verified.verified);
    assert!((searched.cost - verified.cost).abs() > TOLERANCE);
    let mut archive = Archive::default();
    let found = Record::new(program, &searched, true, 0).prove(whole(setting.goal));
    archive.offer(&task.name, found.clone());
    let problem = Problem::new(task.clone(), &setting, ATOM).unwrap();
    renew(&mut archive, &problem, &setting);
    assert_eq!(best(&archive, &task), Some(found));
}

#[test]
fn confirmation() {
    let setting = Setting {
        budget: 1,
        sample: 0,
        ..Setting::default()
    };
    let (task, program) = task("choice", "[A] B, [A] C", "A", "B");
    let searched = evaluate(&program, &task.example, &task.vocabulary, &setting);
    assert!(searched.correct);
    let mut archive = Archive::default();
    archive.offer(&task.name, Record::new(program.clone(), &searched, true, 0));
    assert!(best(&archive, &task).is_some());
    let problem = Problem::new(task.clone(), &setting, ATOM).unwrap();
    renew(&mut archive, &problem, &setting);
    assert_eq!(best(&archive, &task), None);
    assert!(
        archive
            .partial(&task.name)
            .is_none_or(|record| record.program != program)
    );
}
