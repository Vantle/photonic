use crate::argument::Course;
use crate::output::{line, report};
use crate::setup::{open, pool, session, setting};
use learning::attempt;
use learning::curriculum::{Curriculum, Mark, examine, focus, grade, prefix};
use learning::guide::Network;
use learning::home;
use learning::pool::SYNTHETIC;
use learning::session;
use learning::solution::Budget;
use miette::IntoDiagnostic;
use std::time::{Duration, Instant};

fn remaining(deadline: Option<Instant>, time: Duration) -> Duration {
    deadline.map_or(time, |deadline| {
        time.min(deadline.saturating_duration_since(Instant::now()))
    })
}

pub fn run(argument: Course) -> miette::Result<()> {
    let home = open(&argument.home)?;
    let setting = setting(&argument.session, false)?;
    let objective = setting.play.objective;
    let seed = argument.session.seed;
    let deadline = setting.duration.map(|duration| Instant::now() + duration);
    let expired = || deadline.is_some_and(|deadline| Instant::now() >= deadline);
    let mut state: Curriculum = home
        .load(home::CURRICULUM)
        .into_diagnostic()?
        .unwrap_or_default();
    while !expired() && state.level <= argument.level {
        let level = state.level;
        let mut tried = 0;
        while state.exam(level).len() < argument.exam && tried < 20 * argument.exam && !expired() {
            tried += 1;
            let task = state.breed(level, seed, &objective).into_diagnostic()?;
            let time = remaining(deadline, Duration::from_secs(argument.enumerate));
            if let Some(exam) = grade(task, &objective, time).into_diagnostic()? {
                state.exam(level).push(exam);
            }
        }
        home.save(home::CURRICULUM, &state).into_diagnostic()?;
        if expired() {
            break;
        }
        if state.exam(level).is_empty() {
            line(&format!(
                "level {level}: no behavior was proven optimal within {}s; raise --enumerate to continue",
                argument.enumerate
            ));
            break;
        }
        let known = pool(&home, SYNTHETIC, 0, seed, &objective)?;
        let have = known
            .iter()
            .filter(|task| task.name.starts_with(&prefix(level)))
            .count();
        let fresh = (have..argument.fresh)
            .map(|_| state.breed(level, seed, &objective))
            .collect::<Result<Vec<_>, _>>()
            .into_diagnostic()?;
        let named = fresh
            .iter()
            .map(|task| task.name.clone())
            .collect::<Vec<_>>();
        let pool = known.into_iter().chain(fresh).collect::<Vec<_>>();
        home.save(home::POOL, &pool).into_diagnostic()?;
        home.save(home::CURRICULUM, &state).into_diagnostic()?;
        attempt::run(
            &home,
            &pool,
            &named,
            &attempt::Setting {
                objective,
                bound: setting.play.bound,
                budget: Budget {
                    time: Some(Duration::from_secs(argument.enumerate)),
                    ..Budget::default()
                },
                guide: Duration::from_secs(argument.guide),
                blind: true,
                deadline,
            },
            |event| report(event, 0),
        )
        .into_diagnostic()?;
        let practice = remaining(deadline, Duration::from_secs(argument.practice));
        if practice.is_zero() {
            break;
        }
        let (count, focus) = focus(
            &pool,
            level,
            argument.rehearse,
            seed ^ state.mark.len() as u64,
        );
        line(&format!(
            "level {level}: training {:.0}s on {count} behaviors of this level and {} from earlier levels",
            practice.as_secs_f64(),
            focus.len() - count
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
        let mut network = Network::load(&home.file(home::CHECKPOINT)).into_diagnostic()?;
        let mastery = (1..=level)
            .map(|grade| {
                examine(
                    state.exam(grade),
                    &mut network,
                    &objective,
                    argument.expansion,
                    deadline,
                )
            })
            .collect::<Vec<_>>();
        if expired() {
            break;
        }
        let round = state.mark.len() + 1;
        line(&format!(
            "round {round}: level {level}; optimal within {} programs on {}",
            argument.expansion,
            mastery
                .iter()
                .enumerate()
                .map(|(index, share)| format!("level {} {:.0}%", index + 1, share * 100.0))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        if mastery[level - 1] >= argument.mastery {
            line(&format!("level {level} mastered"));
            state.level += 1;
        }
        state.mark.push(Mark {
            round,
            level,
            mastery,
        });
        home.save(home::CURRICULUM, &state).into_diagnostic()?;
    }
    Ok(())
}
