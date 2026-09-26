use crate::argument::Improve;
use crate::output::{line, proof, report};
use crate::setup::{archive, open, pool, session, setting};
use hashing::combine;
use learning::attempt;
use learning::home;
use learning::pool::{self, SYNTHETIC};
use learning::session;
use learning::solution::Budget;
use learning::task::Task;
use miette::IntoDiagnostic;
use std::time::{Duration, Instant};

pub fn run(argument: Improve) -> miette::Result<()> {
    let home = open(&argument.home)?;
    let setting = setting(&argument.session, false)?;
    let deadline = setting.duration.map(|duration| Instant::now() + duration);
    let option = attempt::Setting {
        objective: setting.play.objective,
        bound: setting.play.bound,
        budget: Budget {
            time: Some(Duration::from_secs(argument.enumerate)),
            ..Budget::default()
        },
        guide: Duration::from_secs(argument.guide),
        blind: true,
        deadline,
    };
    for round in 0..argument.round {
        if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            break;
        }
        let known = pool(
            &home,
            SYNTHETIC,
            0,
            argument.session.seed,
            &setting.play.objective,
        )?;
        let before = known.len();
        let grown = pool::grow(
            known,
            argument.fresh,
            combine(argument.session.seed, round as u64),
            &setting.play.objective,
        )
        .into_diagnostic()?;
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
        let count = attempt::run(&home, &pool, &fresh, &option, |event| report(event, 0))
            .into_diagnostic()?;
        line(&format!(
            "round {}: solved {} of {} new behaviors from their tests before training on them, {}, {} passing held-out tests, in {:.1}s",
            round + 1,
            count.found,
            count.total,
            proof(&count),
            count.general,
            start.elapsed().as_secs_f64()
        ));
        let archive = archive(&home)?;
        let focus = pool
            .iter()
            .filter(|task| archive.best(&task.name).is_none())
            .map(|task| task.name.clone())
            .collect::<Vec<_>>();
        let practice = deadline.map_or(Duration::from_secs(argument.practice), |deadline| {
            Duration::from_secs(argument.practice)
                .min(deadline.saturating_duration_since(Instant::now()))
        });
        if practice.is_zero() {
            break;
        }
        line(&format!(
            "round {}: training for {:.0}s, focused on the {} tasks still unsolved",
            round + 1,
            practice.as_secs_f64(),
            focus.len()
        ));
        session(
            &home,
            &pool,
            &focus,
            &session::Setting {
                duration: Some(practice),
                ..setting
            },
        )?;
    }
    Ok(())
}
