use crate::argument;
use crate::output::{line, observe};
use learning::archive::Archive;
use learning::edit::Bound;
use learning::encoding::{DIMENSION, Shape};
use learning::export::source;
use learning::home::{self, Home};
use learning::import;
use learning::objective;
use learning::play;
use learning::pool;
use learning::search;
use learning::session;
use learning::task::Task;
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

pub fn setting(session: &argument::Session) -> session::Setting {
    session::Setting {
        placement: match session.device {
            argument::Device::Auto => session::Placement::Automatic,
            argument::Device::Gpu => session::Placement::Graphics,
            argument::Device::Cpu => session::Placement::Processor,
        },
        worker: session.worker,
        trainer: session.trainer,
        frozen: session.frozen,
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
                processor: session.processor,
                size: session.size,
                ..objective::Setting::default()
            },
            infer: session.infer,
            race: session.race,
            ..play::Setting::default()
        },
        train: train::Setting::default(),
    }
}

pub fn pool(
    home: &Home,
    synthetic: usize,
    grow: usize,
    seed: u64,
    objective: &objective::Setting,
) -> miette::Result<Vec<Task>> {
    let pool = match home.load::<Vec<Task>>(home::POOL).into_diagnostic()? {
        Some(pool) => pool::grow(pool::merge(pool), grow, seed, objective),
        None => pool::initial(synthetic, seed, objective),
    };
    home.save(home::POOL, &pool).into_diagnostic()?;
    Ok(pool)
}

pub fn export(home: &Home, pool: &[Task]) -> miette::Result<()> {
    let archive: Archive = home
        .load(home::ARCHIVE)
        .into_diagnostic()?
        .unwrap_or_default();
    for task in pool {
        let Some(record) = archive
            .entry(&task.name)
            .and_then(|entry| entry.best.as_ref())
        else {
            continue;
        };
        home.write(
            &format!("{}/{}.wave", home::PROGRAM, task.name),
            &source(task, &record.program),
        )
        .into_diagnostic()?;
    }
    Ok(())
}

pub fn session(
    home: &Home,
    pool: &[Task],
    focus: &[String],
    setting: &session::Setting,
) -> miette::Result<()> {
    let (problem, failure) = pool::prepare(pool, &setting.play.objective);
    for error in failure {
        line(&format!("skipped: {error}"));
    }
    let mut chosen = Vec::new();
    for name in focus {
        let index = problem
            .iter()
            .position(|entry| entry.task.name == *name)
            .ok_or_else(|| miette!("no usable task is named {name}"))?;
        chosen.push(index);
    }
    line(&format!(
        "pool: {} problems, {} games per self-play thread",
        problem.len(),
        setting.play.game
    ));
    session::run(home, problem, chosen, setting, observe).into_diagnostic()?;
    export(home, pool)
}
