use crate::argument::{Status, Verify};
use crate::output::line;
use crate::setup::{archive, open};
use code::program::Program;
use learning::archive::{Archive, Coverage, Proof};
use learning::export;
use learning::home;
use learning::objective;
use learning::task::{self, Task};
use miette::IntoDiagnostic;

fn standing(proof: Option<Proof>, verified: bool) -> &'static str {
    match (proof.map(|proof| proof.coverage), verified) {
        (Some(Coverage::Whole), _) => "proven optimal",
        (Some(Coverage::Flat), _) => "proven optimal among flat programs",
        (None, true) => "exhaustive",
        (None, false) => "sampled",
    }
}

pub fn status(argument: &Status) -> miette::Result<()> {
    let home = open(&argument.home)?;
    let pool = home
        .load::<Vec<Task>>(home::POOL)
        .into_diagnostic()?
        .unwrap_or_default();
    let archive = archive(&home)?;
    if let Some(name) = &argument.task {
        let setting = objective::Setting {
            goal: argument.objective.goal().into_diagnostic()?,
            ..objective::Setting::default()
        };
        return detail(&pool, &archive, name, &setting);
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
            standing(record.proof, record.verified)
        ));
    }
    Ok(())
}

fn measure(label: &str, task: &Task, program: &Program, setting: &objective::Setting) -> String {
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
        &setting.thorough().aim(task.goal),
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
        export::source(task, program)
    )
}

fn detail(
    pool: &[Task],
    archive: &Archive,
    name: &str,
    setting: &objective::Setting,
) -> miette::Result<()> {
    let task = task::find(pool, name).into_diagnostic()?;
    match &task.reference {
        Some(reference) => line(&measure("reference", task, reference, setting)),
        None => line("no reference: the task is defined by its tests alone"),
    }
    match archive.best(name) {
        Some(record) => line(&measure("best", task, &record.program, setting)),
        None => line("no correct program recorded yet"),
    }
    Ok(())
}

pub fn verify(argument: &Verify) -> miette::Result<()> {
    let home = open(&argument.home)?;
    let pool = home
        .load::<Vec<Task>>(home::POOL)
        .into_diagnostic()?
        .unwrap_or_default();
    let archive = archive(&home)?;
    let setting = objective::Setting::default().thorough();
    let chosen = match &argument.task {
        Some(name) => vec![task::find(&pool, name).into_diagnostic()?],
        None => pool.iter().collect(),
    };
    for task in chosen {
        let Some(record) = archive.best(&task.name) else {
            if argument.task.is_some() {
                line(&format!("{}: no correct program recorded yet", task.name));
            }
            continue;
        };
        let result = export::verify(task, &record.program, &setting, argument.budget);
        line(&format!(
            "{:<16} kernel {}/{} prism {}/{}",
            task.name, result.kernel, result.total, result.prism, result.expressible
        ));
    }
    Ok(())
}
