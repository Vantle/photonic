use crate::argument::{Optimize, Train};
use crate::output::line;
use crate::setup::{archive, open, pool, read, session, setting};
use learning::export::{BUDGET, source, verify};
use learning::home;
use learning::import::import;
use learning::objective;
use learning::pool::{self, SYNTHETIC};
use learning::task;
use miette::IntoDiagnostic;

pub fn train(argument: Train) -> miette::Result<()> {
    let home = open(&argument.home)?;
    let setting = setting(&argument.session, argument.frozen)?;
    let pool = pool(
        &home,
        argument.synthetic,
        argument.fresh,
        argument.session.seed,
        &setting.play.objective,
    )?;
    session(&home, &pool, &argument.task, &setting)
}

pub fn optimize(argument: Optimize) -> miette::Result<()> {
    let home = open(&argument.home)?;
    let setting = setting(&argument.session, argument.frozen)?;
    let program = argument
        .program
        .iter()
        .map(|path| read(path))
        .collect::<miette::Result<Vec<_>>>()?;
    let input = argument
        .input
        .iter()
        .map(|path| read(path))
        .collect::<miette::Result<Vec<_>>>()?;
    let name = argument.name.clone().unwrap_or_else(|| {
        argument.program[0].file_stem().map_or_else(
            || "program".to_owned(),
            |stem| stem.to_string_lossy().into_owned(),
        )
    });
    let task =
        import(&name, &program, &input, &setting.play.objective.thorough()).into_diagnostic()?;
    if let Some(reference) = &task.reference {
        let size = objective::Size::new(reference);
        line(&format!(
            "imported {name}: {} rules, {} examples, reference size {}",
            size.rule,
            task.example.len(),
            size.total()
        ));
    }
    let existing = pool(
        &home,
        SYNTHETIC,
        0,
        argument.session.seed,
        &setting.play.objective,
    )?;
    let pool = pool::admit(existing, task, &setting.play.objective).into_diagnostic()?;
    home.save(home::POOL, &pool).into_diagnostic()?;
    session(&home, &pool, std::slice::from_ref(&name), &setting)?;
    let archive = archive(&home)?;
    let task = task::find(&pool, &name).into_diagnostic()?;
    match archive.best(&name) {
        Some(record) => {
            line(&format!(
                "best {name}: cost {:.3}, size {}, time {:.3} (work {:.2}, span {:.2})",
                record.cost, record.size, record.time, record.work, record.span
            ));
            line(&source(task, &record.program));
            let result = verify(
                task,
                &record.program,
                &setting.play.objective.thorough(),
                BUDGET,
            );
            line(&format!(
                "verified by Photonic: kernel {}/{} examples, prism {}/{} expressible targets",
                result.kernel, result.total, result.prism, result.expressible
            ));
        }
        None => line(&format!("no correct program recorded for {name} yet")),
    }
    Ok(())
}
