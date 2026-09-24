use crate::argument::Course;
use crate::output::line;
use crate::setup::{open, pool, session, setting};
use crate::solve::{Attempt, attempt};
use learning::curriculum::{Curriculum, Mark, examine, focus, grade, prefix};
use learning::guide::Network;
use learning::home;
use learning::session;
use learning::solution::Budget;
use miette::IntoDiagnostic;
use std::time::{Duration, Instant};

pub fn run(argument: Course) -> miette::Result<()> {
    let home = open(&argument.home)?;
    let setting = setting(&argument.session);
    let objective = setting.play.objective;
    let seed = argument.session.seed;
    let mut state: Curriculum = home
        .load(home::CURRICULUM)
        .into_diagnostic()?
        .unwrap_or_default();
    let start = Instant::now();
    let limit = argument.session.duration.map(Duration::from_secs);
    while limit.is_none_or(|limit| start.elapsed() < limit) && state.level <= argument.levels {
        let level = state.level;
        let mut tried = 0;
        while state.exam(level).len() < argument.exam && tried < 20 * argument.exam {
            tried += 1;
            let task = state.breed(level, seed, &objective);
            if let Some(exam) = grade(task, &objective, Duration::from_secs(argument.grade)) {
                state.exam(level).push(exam);
            }
        }
        home.save(home::CURRICULUM, &state).into_diagnostic()?;
        if state.exam(level).is_empty() {
            line(&format!(
                "level {level}: no behavior was proven optimal within {}s; raise --grade to continue",
                argument.grade
            ));
            break;
        }
        let known = pool(&home, 48, 0, seed, &objective)?;
        let have = known
            .iter()
            .filter(|task| task.name.starts_with(&prefix(level)))
            .count();
        let fresh = (have..argument.train)
            .map(|_| state.breed(level, seed, &objective))
            .collect::<Vec<_>>();
        let named = fresh
            .iter()
            .map(|task| task.name.clone())
            .collect::<Vec<_>>();
        let pool = known.into_iter().chain(fresh).collect::<Vec<_>>();
        home.save(home::POOL, &pool).into_diagnostic()?;
        home.save(home::CURRICULUM, &state).into_diagnostic()?;
        if !named.is_empty() {
            attempt(
                &home,
                &pool,
                &named,
                &Attempt {
                    budget: Budget {
                        size: 16,
                        time: Some(Duration::from_secs(argument.grade)),
                    },
                    guide: argument.guide,
                    blind: true,
                    show: 0,
                },
            )?;
        }
        let (count, focus) = focus(
            &pool,
            level,
            argument.rehearse,
            seed ^ state.mark.len() as u64,
        );
        line(&format!(
            "level {level}: training {}s on {count} behaviors of this level and {} from earlier levels",
            argument.round,
            focus.len() - count
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
        let mut network = Network::load(&home.file(home::CHECKPOINT)).into_diagnostic()?;
        let mastery = (1..=level)
            .map(|grade| {
                examine(
                    state.exam(grade),
                    &mut network,
                    &objective,
                    argument.expansion,
                )
            })
            .collect::<Vec<_>>();
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
