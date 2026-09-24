use crate::argument::Improve;
use crate::output::line;
use crate::setup::{open, pool, session, setting};
use crate::solve::{Attempt, attempt};
use learning::archive::Archive;
use learning::home;
use learning::pool;
use learning::session;
use learning::solution::Budget;
use learning::task::Task;
use miette::IntoDiagnostic;
use std::time::{Duration, Instant};

pub fn run(argument: Improve) -> miette::Result<()> {
    let home = open(&argument.home)?;
    let setting = setting(&argument.session);
    let option = Attempt {
        budget: Budget {
            size: 16,
            time: Some(Duration::from_secs(argument.budget)),
        },
        guide: argument.guide,
        blind: true,
        show: 0,
    };
    for round in 0..argument.rounds {
        let known = pool(&home, 48, 0, argument.session.seed, &setting.play.objective)?;
        let before = known.len();
        let grown = pool::grow(
            known,
            argument.fresh,
            argument.session.seed ^ round as u64,
            &setting.play.objective,
        );
        let fresh = grown[before..]
            .iter()
            .map(|task| task.name.clone())
            .collect::<Vec<_>>();
        let pool = grown
            .into_iter()
            .map(|task| {
                if fresh.contains(&task.name) {
                    Task {
                        reference: None,
                        ..task
                    }
                } else {
                    task
                }
            })
            .collect::<Vec<_>>();
        home.save(home::POOL, &pool).into_diagnostic()?;
        let start = Instant::now();
        let count = attempt(&home, &pool, &fresh, &option)?;
        line(&format!(
            "round {}: solved {} of {} new behaviors from their tests before training on them, {} proven optimal, {} passing held-out tests, in {:.1}s",
            round + 1,
            count.found,
            count.total,
            count.proven,
            count.general,
            start.elapsed().as_secs_f64()
        ));
        let archive: Archive = home
            .load(home::ARCHIVE)
            .into_diagnostic()?
            .unwrap_or_default();
        let focus = pool
            .iter()
            .filter(|task| {
                archive
                    .entry(&task.name)
                    .and_then(|entry| entry.best.as_ref())
                    .is_none()
            })
            .map(|task| task.name.clone())
            .collect::<Vec<_>>();
        line(&format!(
            "round {}: training for {}s, focused on the {} tasks still unsolved",
            round + 1,
            argument.round,
            focus.len()
        ));
        session(
            &home,
            &pool,
            &focus,
            &session::Setting {
                duration: Some(Duration::from_secs(argument.round)),
                ..setting
            },
        )?;
    }
    Ok(())
}
