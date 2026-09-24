use crate::argument::{Optimize, Train};
use crate::output::line;
use crate::setup::{open, pool, read, session, setting};
use learning::archive::Archive;
use learning::export::{source, verify};
use learning::home;
use learning::import::import;
use learning::objective;
use learning::pool;
use miette::{IntoDiagnostic, miette};

pub fn train(argument: Train) -> miette::Result<()> {
    let home = open(&argument.home)?;
    let setting = setting(&argument.session);
    let pool = pool(
        &home,
        argument.synthetic,
        argument.grow,
        argument.session.seed,
        &setting.play.objective,
    )?;
    session(&home, &pool, &argument.task, &setting)
}

pub fn optimize(argument: Optimize) -> miette::Result<()> {
    let home = open(&argument.home)?;
    let mut setting = setting(&argument.session);
    setting.play.focus = argument.focus;
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
        line(&format!(
            "imported {name}: {} rules, {} examples, reference size {}",
            code::tree::walk(reference).len(),
            task.example.len(),
            objective::Size::new(reference).total()
        ));
    }
    let existing = pool(&home, 48, 0, argument.session.seed, &setting.play.objective)?;
    let pool = pool::admit(existing, task);
    home.save(home::POOL, &pool).into_diagnostic()?;
    session(&home, &pool, std::slice::from_ref(&name), &setting)?;
    let archive: Archive = home
        .load(home::ARCHIVE)
        .into_diagnostic()?
        .unwrap_or_default();
    let task = pool
        .iter()
        .find(|task| task.name == name)
        .ok_or_else(|| miette!("the pool lost {name}"))?;
    match archive.entry(&name).and_then(|entry| entry.best.as_ref()) {
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
                2_000_000,
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
