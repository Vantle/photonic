use crate::archive::{Archive, Record};
use crate::objective::{Evaluation, Setting, atom, evaluate, general};
use crate::problem::Problem;
use crate::task::Task;
use crate::tree::walk;
use code::program::Program;

fn fits(program: &Program, task: &Task) -> bool {
    atom(&walk(program))
        .last()
        .is_none_or(|atom| atom.index() < task.vocabulary.len())
}

fn measure(program: &Program, task: &Task, setting: &Setting) -> Option<Evaluation> {
    if !fits(program, task) {
        return None;
    }
    let evaluation = evaluate(program, &task.example, &task.vocabulary, setting);
    if !evaluation.correct {
        return Some(evaluation);
    }
    let verified = evaluate(
        program,
        &task.example,
        &task.vocabulary,
        &setting.thorough(),
    );
    (verified.correct && general(program, task, setting)).then_some(evaluation)
}

pub fn renew(archive: &mut Archive, problem: &Problem, setting: &Setting) {
    let task = &problem.task;
    let setting = setting.aim(task.goal);
    let prior = archive.register(&task.name, problem.baseline);
    let reference = task.reference.clone().and_then(|program| {
        let evaluation = measure(&program, task, &setting)?;
        Some(Record::new(program, &evaluation, evaluation.correct, 0))
    });
    let known = prior
        .best
        .into_iter()
        .chain(prior.partial)
        .filter_map(|record| {
            let evaluation = measure(&record.program, task, &setting)?;
            Some(record.revise(&evaluation, evaluation.correct, setting.goal))
        });
    for record in reference.into_iter().chain(known) {
        archive.offer(&task.name, record);
    }
}
