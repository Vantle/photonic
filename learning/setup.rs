use crate::argument;
use crate::output::{line, observe};
use learning::archive::Archive;
use learning::edit::Bound;
use learning::encoding::{DIMENSION, Shape};
use learning::export;
use learning::home::{self, Home};
use learning::import;
use learning::objective;
use learning::play;
use learning::pool;
use learning::search;
use learning::session;
use learning::task::{self, Task};
use learning::train;
use miette::{IntoDiagnostic, WrapErr, miette};
use std::path::{Path, PathBuf};
use std::time::Duration;

fn resolve(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        return path;
    }
    std::env::var_os("BUILD_WORKING_DIRECTORY").map_or_else(
        || path.clone(),
        |directory| PathBuf::from(directory).join(&path),
    )
}

pub fn open(argument: &argument::Home) -> miette::Result<Home> {
    let path = match &argument.home {
        Some(path) => resolve(path.clone()),
        None => std::env::var_os("HOME")
            .map(|home| PathBuf::from(home).join(".cache/photonic/learning"))
            .ok_or_else(|| miette!("pass --home or set HOME"))?,
    };
    Home::open(path).into_diagnostic()
}

pub fn read(path: &Path) -> miette::Result<import::Source> {
    let path = resolve(path.to_path_buf());
    Ok(import::Source {
        text: std::fs::read_to_string(&path)
            .into_diagnostic()
            .wrap_err_with(|| format!("could not read {}", path.display()))?,
        origin: path.display().to_string(),
    })
}

pub fn setting(session: &argument::Session, frozen: bool) -> miette::Result<session::Setting> {
    Ok(session::Setting {
        placement: match session.device {
            argument::Device::Auto => session::Placement::Automatic,
            argument::Device::Gpu => session::Placement::Graphics,
            argument::Device::Cpu => session::Placement::Processor,
        },
        worker: session.worker,
        trainer: session.trainer,
        frozen,
        duration: session.duration.map(Duration::from_secs),
        report: Duration::from_secs(session.report.max(1)),
        seed: session.seed,
        shape: Shape {
            width: session.width,
            depth: session.depth,
            head: session.width / DIMENSION,
            hidden: session.hidden,
            key: DIMENSION,
        },
        capacity: 200_000,
        teach: (session.teach > 0).then(|| Duration::from_secs(session.teach)),
        play: play::Setting {
            game: session.game,
            search: search::Setting {
                simulation: session.simulation,
                considered: session.considered,
                step: session.step,
                ..search::Setting::default()
            },
            bound: Bound {
                depth: session.nesting,
                ..Bound::default()
            },
            objective: objective::Setting {
                goal: session.objective.goal().into_diagnostic()?,
                ..objective::Setting::default()
            },
            focus: session.focus,
            infer: session.infer,
            race: session.race,
            ..play::Setting::default()
        },
        train: train::Setting::default(),
    })
}

pub fn pool(
    home: &Home,
    synthetic: usize,
    fresh: usize,
    seed: u64,
    objective: &objective::Setting,
) -> miette::Result<Vec<Task>> {
    let pool = match home.load::<Vec<Task>>(home::POOL).into_diagnostic()? {
        Some(pool) => pool::grow(pool::merge(pool).into_diagnostic()?, fresh, seed, objective),
        None => pool::initial(synthetic, seed, objective),
    }
    .into_diagnostic()?;
    home.save(home::POOL, &pool).into_diagnostic()?;
    Ok(pool)
}

pub fn archive(home: &Home) -> miette::Result<Archive> {
    Ok(home
        .load(home::ARCHIVE)
        .into_diagnostic()?
        .unwrap_or_default())
}

pub fn session(
    home: &Home,
    pool: &[Task],
    focus: &[String],
    setting: &session::Setting,
) -> miette::Result<()> {
    for name in focus {
        task::find(pool, name).into_diagnostic()?;
    }
    let (problem, failure) = pool::prepare(pool, &setting.play.objective);
    for error in failure {
        line(&format!("skipped: {error}"));
    }
    let chosen = problem
        .iter()
        .enumerate()
        .filter(|(_, entry)| focus.contains(&entry.task.name))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if !focus.is_empty() && chosen.is_empty() {
        return Err(miette!(
            "none of the {} tasks to focus on can be trained on",
            focus.len()
        ));
    }
    line(&format!(
        "pool: {} problems, {} games per self-play thread",
        problem.len(),
        setting.play.game
    ));
    session::run(home, problem, chosen, setting, observe).into_diagnostic()?;
    export::write(home, pool, &archive(home)?).into_diagnostic()
}
