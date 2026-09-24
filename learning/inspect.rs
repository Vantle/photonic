use crate::argument::{Status, Verify};
use crate::output::line;
use crate::setup::open;
use code::program::Program;
use learning::archive::Archive;
use learning::export::{source, verify};
use learning::home;
use learning::objective;
use learning::task::Task;
use miette::{IntoDiagnostic, miette};

pub fn status(argument: &Status) -> miette::Result<()> {
    let home = open(&argument.home)?;
    let pool = home
        .load::<Vec<Task>>(home::POOL)
        .into_diagnostic()?
        .unwrap_or_default();
    let archive: Archive = home
        .load(home::ARCHIVE)
        .into_diagnostic()?
        .unwrap_or_default();
    if let Some(name) = &argument.task {
        return detail(&pool, &archive, name);
    }
    line(&format!("home: {}", home.path().display()));
    line(&format!("pool: {} tasks", pool.len()));
    for (name, entry) in archive.task() {
        let Some(record) = entry.best.as_ref() else {
            continue;
        };
        let saving = 100.0 * (entry.baseline - record.cost) / entry.baseline;
        line(&format!(
            "{name:<16} baseline {:>8.3} best {:>8.3} ({saving:+6.1}%) size {:>4} time {:>7.3} {}",
            entry.baseline,
            record.cost,
            record.size,
            record.time,
            match (record.proven, record.verified) {
                (true, _) => "proven optimal",
                (false, true) => "exhaustive",
                (false, false) => "sampled",
            }
        ));
    }
    Ok(())
}

fn measure(label: &str, task: &Task, program: &Program) -> String {
    let example = task
        .example
        .iter()
        .chain(&task.holdout)
        .cloned()
        .collect::<Vec<_>>();
    let evaluation = objective::evaluate(
        program,
        &example,
        &task.vocabulary,
        &objective::Setting::default().thorough().aim(task.goal),
    );
    format!(
        "{label}: {} rules, size {}, work {:.2}, span {:.2}, time {:.3}, cost {:.3}, {} on {} inputs\n{}",
        evaluation.size.rule,
        evaluation.size.total(),
        evaluation.work,
        evaluation.span,
        evaluation.time,
        evaluation.cost,
        match (evaluation.correct, evaluation.verified) {
            (true, true) => "exhaustively correct",
            (true, false) => "correct by sampling",
            (false, _) => "incorrect",
        },
        example.len(),
        source(task, program)
    )
}

fn detail(pool: &[Task], archive: &Archive, name: &str) -> miette::Result<()> {
    let task = pool
        .iter()
        .find(|task| task.name == name)
        .ok_or_else(|| miette!("no task is named {name}"))?;
    match &task.reference {
        Some(reference) => line(&measure("reference", task, reference)),
        None => line("no reference: the task is defined by its tests alone"),
    }
    match archive.entry(name).and_then(|entry| entry.best.as_ref()) {
        Some(record) => line(&measure("best", task, &record.program)),
        None => line("no correct program recorded yet"),
    }
    Ok(())
}

pub fn check(argument: &Verify) -> miette::Result<()> {
    let home = open(&argument.home)?;
    let pool = home
        .load::<Vec<Task>>(home::POOL)
        .into_diagnostic()?
        .unwrap_or_default();
    let archive: Archive = home
        .load(home::ARCHIVE)
        .into_diagnostic()?
        .unwrap_or_default();
    let setting = objective::Setting::default().thorough();
    for task in &pool {
        if argument
            .task
            .as_ref()
            .is_some_and(|name| *name != task.name)
        {
            continue;
        }
        let Some(record) = archive
            .entry(&task.name)
            .and_then(|entry| entry.best.as_ref())
        else {
            continue;
        };
        let result = verify(task, &record.program, &setting, argument.budget);
        line(&format!(
            "{:<16} kernel {}/{} prism {}/{}",
            task.name, result.kernel, result.total, result.prism, result.expressible
        ));
    }
    Ok(())
}
