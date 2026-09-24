use crate::archive::{Archive, Improvement, Record};
use crate::demonstration::Lesson;
use crate::encoding::Shape;
use crate::home::{self, Home};
use crate::judge::Judge;
use crate::objective::evaluate;
use crate::play::{self, Shared, Statistic, Tally, Worker};
use crate::problem::Problem;
use crate::replay::Replay;
use crate::server::{Server, serve};
use crate::train::{self, Device, Progress, Trainer};
use gpu::engine::Engine;
use network::checkpoint;
use network::grow::{self, grow};
use network::model::Model;
use network::optimizer::Optimizer;
use random::Generator;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Placement {
    Automatic,
    Graphics,
    Processor,
}

#[derive(Clone, Copy, Debug)]
pub struct Setting {
    pub placement: Placement,
    pub worker: Option<usize>,
    pub trainer: usize,
    pub frozen: bool,
    pub duration: Option<Duration>,
    pub report: Duration,
    pub seed: u64,
    pub shape: Shape,
    pub capacity: usize,
    pub teach: Option<Duration>,
    pub play: play::Setting,
    pub train: train::Setting,
}

#[derive(Debug, Error)]
pub enum Failure {
    #[error(transparent)]
    Home(#[from] home::Failure),
    #[error(transparent)]
    Checkpoint(#[from] checkpoint::Failure),
    #[error("could not build the training pool: {0}")]
    Pool(#[from] rayon::ThreadPoolBuildError),
    #[error("the pool holds no usable problem")]
    Empty,
    #[error(transparent)]
    Graphics(#[from] gpu::failure::Failure),
    #[error("the saved network cannot read the current encoding: {0}")]
    Interface(grow::Failure),
}

#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct Report {
    pub elapsed: f64,
    pub episode: u64,
    pub step: u64,
    pub evaluation: u64,
    pub inferred: u64,
    pub skipped: u64,
    pub inference: u64,
    pub sample: u64,
    pub train: u64,
    pub policy: f64,
    pub value: f64,
    pub entropy: f64,
    pub norm: f64,
    pub fit: f64,
    pub judge: f64,
    pub risk: f64,
    pub reward: f64,
    pub correct: f64,
    pub improvement: u64,
}

#[derive(Clone, Copy, Default)]
struct Window {
    tally: Tally,
    judge: Judge,
}

#[derive(Serialize)]
struct Entry<'improvement> {
    improvement: &'improvement Improvement,
    source: String,
}

fn entry<'improvement>(
    shared: &Shared,
    improvement: &'improvement Improvement,
) -> Entry<'improvement> {
    Entry {
        improvement,
        source: shared
            .problem
            .iter()
            .find(|problem| problem.task.name == improvement.task)
            .map(|problem| {
                translation::text::program(&improvement.record.program, &problem.task.vocabulary)
            })
            .unwrap_or_default(),
    }
}

pub enum Origin {
    Fresh,
    Restored,
    Grown { from: usize },
    Kept(grow::Failure),
}

pub enum Event {
    Start {
        parameter: usize,
        origin: Origin,
        inference: String,
        training: Option<String>,
        worker: usize,
    },
    Lesson {
        count: usize,
    },
    Report(Report),
    Discovery(Improvement),
}

fn prepare(home: &Home, setting: &Setting) -> Result<(Model, Optimizer, Origin), Failure> {
    let path = home.file(home::CHECKPOINT);
    let architecture = setting.shape.architecture();
    let mut generator = Generator::new(setting.seed);
    if !path.exists() {
        let model = Model::new(architecture, &mut generator);
        let optimizer = Optimizer::new(
            model.size(),
            network::optimizer::Setting {
                rate: setting.train.rate,
                ..network::optimizer::Setting::default()
            },
        );
        return Ok((model, optimizer, Origin::Fresh));
    }
    let (model, optimizer) = checkpoint::load(&path)?;
    let saved = model.configuration().clone();
    if saved == architecture {
        return Ok((model, optimizer, Origin::Restored));
    }
    let compatible = saved.field == architecture.field
        && saved.unary == architecture.unary
        && saved.binary == architecture.binary
        && saved.key == architecture.key;
    match grow(&model, architecture, &mut generator) {
        Ok(grown) => {
            let optimizer = Optimizer::new(grown.size(), optimizer.setting);
            Ok((grown, optimizer, Origin::Grown { from: model.size() }))
        }
        Err(failure) if compatible => Ok((model, optimizer, Origin::Kept(failure))),
        Err(failure) => Err(Failure::Interface(failure)),
    }
}

fn engine(model: &Model, placement: Placement) -> Result<Option<Engine>, Failure> {
    match placement {
        Placement::Processor => Ok(None),
        Placement::Graphics => Ok(Some(Engine::new(model)?)),
        Placement::Automatic => Ok(Engine::new(model).ok()),
    }
}

fn device(model: &Model, setting: &Setting) -> Result<Option<Device>, Failure> {
    if setting.frozen {
        return Ok(None);
    }
    if let Some(engine) = engine(model, setting.placement)? {
        return Ok(Some(Device::Graphics(Box::new(engine))));
    }
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(setting.trainer.max(1))
        .build()?;
    Ok(Some(Device::Processor(pool)))
}

fn report(
    shared: &Shared,
    progress: &Mutex<Progress>,
    start: Instant,
    previous: &mut Window,
) -> Report {
    let statistic: &Statistic = &shared.statistic;
    let total = Window {
        tally: *statistic
            .tally
            .lock()
            .expect("the tally lock is never poisoned"),
        judge: *statistic
            .judge
            .lock()
            .expect("the judge lock is never poisoned"),
    };
    let tally = Tally {
        reward: total.tally.reward - previous.tally.reward,
        episode: total.tally.episode - previous.tally.episode,
        correct: total.tally.correct - previous.tally.correct,
    };
    let checked = total.judge.count - previous.judge.count;
    let judge = if checked == 0 {
        f64::NAN
    } else {
        (total.judge.total - previous.judge.total) / checked as f64
    };
    let worthy = total.judge.recall.worthy - previous.judge.recall.worthy;
    let risk = if worthy <= 0.0 {
        f64::NAN
    } else {
        (total.judge.recall.lost - previous.judge.recall.lost) / worthy
    };
    *previous = total;
    let progress = *progress
        .lock()
        .expect("the progress lock is never poisoned");
    let sample = shared
        .replay
        .lock()
        .expect("the replay lock is never poisoned")
        .total();
    let episode = tally.episode.max(1) as f64;
    Report {
        elapsed: start.elapsed().as_secs_f64(),
        episode: statistic.episode.load(Ordering::Relaxed),
        step: statistic.step.load(Ordering::Relaxed),
        evaluation: statistic.evaluation.load(Ordering::Relaxed),
        inferred: statistic.inferred.load(Ordering::Relaxed),
        skipped: total.judge.skipped,
        inference: statistic.inference.load(Ordering::Relaxed),
        sample,
        train: progress.step,
        policy: progress.policy,
        value: progress.value,
        entropy: progress.entropy,
        norm: progress.norm,
        fit: progress.judge,
        judge,
        risk,
        reward: tally.reward / episode,
        correct: tally.correct as f64 / episode,
        improvement: statistic.improvement.load(Ordering::Relaxed),
    }
}

pub fn run(
    home: &Home,
    problem: Vec<Problem>,
    focus: Vec<usize>,
    setting: &Setting,
    mut observe: impl FnMut(Event),
) -> Result<Report, Failure> {
    if problem.is_empty() {
        return Err(Failure::Empty);
    }
    let (model, optimizer, origin) = prepare(home, setting)?;
    let engine = engine(&model, setting.placement)?;
    let device = device(&model, setting)?;
    let (reserved, training) = match &device {
        None => (0, None),
        Some(Device::Processor(_)) => (
            setting.trainer,
            Some(format!("{} CPU threads", setting.trainer.max(1))),
        ),
        Some(Device::Graphics(engine)) => (1, Some(engine.name().to_owned())),
    };
    let core = std::thread::available_parallelism().map_or(8, std::num::NonZero::get);
    let playing = core.saturating_sub(reserved).max(1);
    let worker = setting.worker.unwrap_or(if engine.is_some() {
        2 * playing
    } else {
        playing
    });
    observe(Event::Start {
        parameter: model.size(),
        origin,
        inference: engine
            .as_ref()
            .map_or_else(|| "CPU".to_owned(), |engine| engine.name().to_owned()),
        training,
        worker,
    });
    let (server, receiver) = Server::new();
    let mut archive: Archive = home.load(home::ARCHIVE)?.unwrap_or_default();
    let thorough = setting.play.objective.thorough();
    for entry in &problem {
        let task = &entry.task;
        let prior = archive.forget(&task.name);
        archive.register(
            &task.name,
            entry.baseline,
            task.reference
                .clone()
                .zip(entry.reference.as_ref())
                .map(|(program, evaluation)| Record::new(program, evaluation, true, 0)),
        );
        if let Some(record) = prior {
            let evaluation = evaluate(
                &record.program,
                &task.example,
                &task.vocabulary,
                &thorough.aim(task.goal),
            );
            let general =
                evaluate(&record.program, &task.holdout, &task.vocabulary, &thorough).correct;
            if evaluation.correct {
                archive.offer(
                    &task.name,
                    Record::new(record.program, &evaluation, general, record.moment),
                );
            }
        }
    }
    let shared = Arc::new(Shared {
        model: RwLock::new(Arc::new(model.clone())),
        server: engine.is_some().then_some(server),
        replay: Mutex::new(Replay::new(setting.capacity)),
        archive: Mutex::new(archive),
        problem,
        focus,
        statistic: Statistic::default(),
        stop: AtomicBool::new(false),
        discovery: Mutex::new(Vec::new()),
    });
    let mut lesson = setting.teach.map(|_| {
        let archive = shared
            .archive
            .lock()
            .expect("the archive lock is never poisoned")
            .clone();
        Lesson::new(&shared.problem, &archive, &setting.play)
    });
    if let Some(lesson) = &lesson {
        observe(Event::Lesson {
            count: lesson.count(),
        });
    }
    let mut teacher = Generator::new(setting.seed ^ 0x7eac);
    let mut trainer = device
        .map(|device| {
            Trainer::new(
                model,
                optimizer,
                device,
                setting.train,
                setting.seed ^ 0x5eed,
            )
        })
        .transpose()?;
    let progress = Mutex::new(Progress::default());
    let failure: Mutex<Option<Failure>> = Mutex::new(None);
    let path = home.file(home::CHECKPOINT);
    let start = Instant::now();
    let outcome = std::thread::scope(|scope| -> Result<Report, Failure> {
        if let Some(engine) = engine {
            let shared = &shared;
            scope.spawn(move || serve(engine, &receiver, shared));
        }
        for index in 0..worker {
            let shared = shared.clone();
            let seed = setting.seed.wrapping_add(1 + index as u64);
            let play = setting.play;
            scope.spawn(move || Worker::new(shared, play, seed).run());
        }
        if let Some(trainer) = trainer.as_mut() {
            let shared = &shared;
            let progress = &progress;
            let failure = &failure;
            let path = &path;
            scope.spawn(move || {
                let outcome = trainer.run(shared, progress, |current| {
                    if let Err(error) = checkpoint::save(path, current.model(), current.optimizer())
                    {
                        *failure.lock().expect("the failure lock is never poisoned") =
                            Some(Failure::from(error));
                    }
                });
                if let Err(error) = outcome {
                    *failure.lock().expect("the failure lock is never poisoned") =
                        Some(Failure::from(error));
                    shared.stop.store(true, Ordering::Relaxed);
                }
            });
        }
        let mut reported = Instant::now();
        let mut previous = Window::default();
        let mut taught: Option<Instant> = None;
        let result = loop {
            std::thread::sleep(Duration::from_millis(200));
            let discovery = std::mem::take(
                &mut *shared
                    .discovery
                    .lock()
                    .expect("the discovery lock is never poisoned"),
            );
            let mut failed = None;
            for improvement in discovery {
                if let Some(lesson) = lesson.as_mut()
                    && let Some(index) = shared
                        .problem
                        .iter()
                        .position(|problem| problem.task.name == improvement.task)
                {
                    lesson.change(index);
                }
                if let Err(error) = home.append(home::DISCOVERY, &entry(&shared, &improvement)) {
                    failed = Some(error);
                    break;
                }
                observe(Event::Discovery(improvement));
            }
            if let Some(error) = failed {
                break Err(Failure::from(error));
            }
            if let (Some(lesson), Some(interval)) = (lesson.as_mut(), setting.teach)
                && taught.is_none_or(|moment| moment.elapsed() >= interval)
            {
                taught = Some(Instant::now());
                let archive = shared
                    .archive
                    .lock()
                    .expect("the archive lock is never poisoned")
                    .clone();
                lesson.refresh(&shared.problem, &archive, &setting.play);
                let sample = lesson.sample(&shared.problem, &mut teacher);
                let mut replay = shared
                    .replay
                    .lock()
                    .expect("the replay lock is never poisoned");
                for sample in sample {
                    replay.push(sample);
                }
            }
            if reported.elapsed() >= setting.report {
                reported = Instant::now();
                let line = report(&shared, &progress, start, &mut previous);
                let archive = shared
                    .archive
                    .lock()
                    .expect("the archive lock is never poisoned")
                    .clone();
                if let Err(error) = home
                    .append(home::PROGRESS, &line)
                    .and_then(|()| home.save(home::ARCHIVE, &archive))
                {
                    break Err(Failure::from(error));
                }
                observe(Event::Report(line));
            }
            if shared.stop.load(Ordering::Relaxed)
                || setting
                    .duration
                    .is_some_and(|duration| start.elapsed() >= duration)
            {
                break Ok(report(&shared, &progress, start, &mut previous));
            }
        };
        shared.stop.store(true, Ordering::Relaxed);
        result
    });
    let archive = shared
        .archive
        .lock()
        .expect("the archive lock is never poisoned")
        .clone();
    home.save(home::ARCHIVE, &archive)?;
    if let Some(error) = failure
        .lock()
        .expect("the failure lock is never poisoned")
        .take()
    {
        return Err(error);
    }
    if let Some(trainer) = &trainer {
        checkpoint::save(&path, trainer.model(), trainer.optimizer())?;
    }
    let remaining = std::mem::take(
        &mut *shared
            .discovery
            .lock()
            .expect("the discovery lock is never poisoned"),
    );
    for improvement in remaining {
        home.append(home::DISCOVERY, &entry(&shared, &improvement))?;
        observe(Event::Discovery(improvement));
    }
    outcome
}
