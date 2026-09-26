use crate::archive::{Archive, Coverage, Improvement, Proof, Record, moment};
use crate::edit::Bound;
use crate::export;
use crate::guide::{self, Effort, Guidance, Network};
use crate::home::{self, Home};
use crate::objective::{self, Evaluation, TOLERANCE};
use crate::play;
use crate::pool;
use crate::problem::{self, Problem};
use crate::renewal::renew;
use crate::solution::{self, Budget, Solution};
use crate::task::{self, Task};
use code::program::Program;
use std::path::Path;
use std::time::{Duration, Instant};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Failure {
    #[error(transparent)]
    Home(#[from] home::Failure),
    #[error(transparent)]
    Network(#[from] guide::Failure),
    #[error(transparent)]
    Task(#[from] task::Failure),
}

#[derive(Clone, Copy, Debug)]
pub struct Setting {
    pub objective: objective::Setting,
    pub bound: Bound,
    pub budget: Budget,
    pub guide: Duration,
    pub blind: bool,
    pub deadline: Option<Instant>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Count {
    pub total: usize,
    pub found: usize,
    pub proven: usize,
    pub flat: usize,
    pub general: usize,
}

impl Count {
    fn merge(self, other: Self) -> Self {
        Self {
            total: self.total + other.total,
            found: self.found + other.found,
            proven: self.proven + other.proven,
            flat: self.flat + other.flat,
            general: self.general + other.general,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cover {
    pub size: Option<usize>,
    pub gap: f64,
}

pub enum Event<'event> {
    Skip(&'event problem::Failure),
    Unguided(&'event Path),
    Refusal {
        task: &'event Task,
        failure: &'event solution::Failure,
    },
    Exhaustion {
        task: &'event Task,
        known: f64,
        solution: &'event Solution,
        elapsed: Duration,
        coverage: Coverage,
    },
    Search {
        task: &'event Task,
        guidance: &'event Guidance,
    },
    Proof {
        task: &'event Task,
        cover: Cover,
        coverage: Coverage,
    },
    Finding {
        task: &'event Task,
        candidate: &'event [(Program, Evaluation)],
    },
    Discovery(&'event Improvement),
    Holdout {
        task: &'event Task,
        passed: bool,
    },
}

struct Finding {
    candidate: Vec<(Program, Evaluation)>,
    proven: bool,
}

struct Plan {
    budget: Budget,
    guide: Duration,
    coverage: Coverage,
}

fn incumbent(problem: &Problem, archive: &Archive, setting: &objective::Setting) -> f64 {
    let task = &problem.task;
    archive
        .best(&task.name)
        .map(|record| &record.program)
        .into_iter()
        .chain(&task.reference)
        .map(|program| {
            objective::evaluate(
                program,
                &task.example,
                &task.vocabulary,
                &setting.aim(task.goal),
            )
        })
        .filter(|evaluation| evaluation.correct)
        .map(|evaluation| evaluation.cost)
        .fold(f64::INFINITY, f64::min)
}

fn network(
    home: &Home,
    guide: Duration,
    observe: &mut impl FnMut(Event<'_>),
) -> Result<Option<Network>, Failure> {
    if guide.is_zero() {
        return Ok(None);
    }
    let checkpoint = home.file(home::CHECKPOINT);
    if !checkpoint.exists() {
        observe(Event::Unguided(home.path()));
        return Ok(None);
    }
    Ok(Some(Network::load(&checkpoint)?))
}

fn exhaust(
    task: &Task,
    known: f64,
    setting: &Setting,
    plan: &Plan,
    observe: &mut impl FnMut(Event<'_>),
) -> (Finding, Option<Cover>) {
    let begun = Instant::now();
    let solution = match solution::solve(task, known, &setting.objective, plan.budget) {
        Ok(solution) => solution,
        Err(failure) => {
            observe(Event::Refusal {
                task,
                failure: &failure,
            });
            let finding = Finding {
                candidate: Vec::new(),
                proven: false,
            };
            let cover = Cover {
                size: None,
                gap: solution::least(task, &setting.objective),
            };
            return (finding, Some(cover));
        }
    };
    observe(Event::Exhaustion {
        task,
        known,
        solution: &solution,
        elapsed: begun.elapsed(),
        coverage: plan.coverage,
    });
    let cover = (!solution.proven).then(|| Cover {
        size: solution.size,
        gap: solution.gap(),
    });
    let finding = Finding {
        proven: solution.proven && solution.cost <= known + TOLERANCE,
        candidate: solution.optimal,
    };
    (finding, cover)
}

fn find(
    entry: &Problem,
    known: f64,
    network: Option<&mut Network>,
    setting: &Setting,
    plan: &Plan,
    observe: &mut impl FnMut(Event<'_>),
) -> Finding {
    let task = &entry.task;
    let (mut finding, cover) = exhaust(task, known, setting, plan, observe);
    if let (Some(network), Some(cover)) = (network, cover)
        && !plan.guide.is_zero()
    {
        let guidance = guide::search(
            task,
            &play::bound(&setting.bound, entry),
            &setting.objective,
            Effort {
                time: plan.guide,
                expansion: u64::MAX,
            },
            cover.gap,
            network,
        );
        observe(Event::Search {
            task,
            guidance: &guidance,
        });
        if let Some((program, evaluation)) = &guidance.best
            && program.flat()
            && evaluation.cost <= cover.gap + TOLERANCE
            && evaluation.cost <= known + TOLERANCE
        {
            finding.proven = true;
            observe(Event::Proof {
                task,
                cover,
                coverage: plan.coverage,
            });
        }
        finding.candidate.extend(guidance.best);
    }
    finding.candidate.sort_by(|left, right| {
        left.1
            .cost
            .total_cmp(&right.1.cost)
            .then_with(|| left.0.cmp(&right.0))
    });
    finding.candidate.dedup_by(|left, right| left.0 == right.0);
    finding
}

fn keep(
    entry: &Problem,
    finding: &Finding,
    archive: &mut Archive,
    setting: &Setting,
    coverage: Coverage,
    observe: &mut impl FnMut(Event<'_>),
) -> bool {
    let task = &entry.task;
    let goal = setting.objective.aim(task.goal).goal;
    observe(Event::Finding {
        task,
        candidate: &finding.candidate,
    });
    let least = finding
        .candidate
        .first()
        .map_or(f64::INFINITY, |(_, evaluation)| evaluation.cost);
    let mut held = false;
    for (program, evaluation) in &finding.candidate {
        let passes = objective::general(program, task, &setting.objective);
        held |= passes;
        let record = Record::new(program.clone(), evaluation, passes, moment());
        let record = if finding.proven && evaluation.cost <= least + TOLERANCE {
            record.prove(Proof { goal, coverage })
        } else {
            record
        };
        if let Some(improvement) = archive.offer(&task.name, record) {
            observe(Event::Discovery(&improvement));
        }
    }
    if !task.holdout.is_empty() && !finding.candidate.is_empty() {
        observe(Event::Holdout { task, passed: held });
    }
    held
}

fn plan(entry: &Problem, setting: &Setting) -> Plan {
    let remaining = setting
        .deadline
        .map(|deadline| deadline.saturating_duration_since(Instant::now()));
    let coverage = if play::bound(&setting.bound, entry).depth == 0 {
        Coverage::Whole
    } else {
        Coverage::Flat
    };
    Plan {
        budget: Budget {
            time: setting.budget.time.into_iter().chain(remaining).min(),
            ..setting.budget
        },
        guide: remaining.map_or(setting.guide, |remaining| remaining.min(setting.guide)),
        coverage,
    }
}

fn settle(
    entry: &Problem,
    archive: &mut Archive,
    network: Option<&mut Network>,
    setting: &Setting,
    observe: &mut impl FnMut(Event<'_>),
) -> Count {
    let plan = plan(entry, setting);
    let known = if setting.blind {
        f64::INFINITY
    } else {
        incumbent(entry, archive, &setting.objective)
    };
    let finding = find(entry, known, network, setting, &plan, observe);
    let held = keep(entry, &finding, archive, setting, plan.coverage, observe);
    Count {
        total: 1,
        found: usize::from(!finding.candidate.is_empty()),
        proven: usize::from(finding.proven && plan.coverage == Coverage::Whole),
        flat: usize::from(finding.proven && plan.coverage == Coverage::Flat),
        general: usize::from(held),
    }
}

pub fn run(
    home: &Home,
    pool: &[Task],
    chosen: &[String],
    setting: &Setting,
    mut observe: impl FnMut(Event<'_>),
) -> Result<Count, Failure> {
    for name in chosen {
        task::find(pool, name)?;
    }
    let selected = pool
        .iter()
        .filter(|task| chosen.contains(&task.name))
        .cloned()
        .collect::<Vec<_>>();
    let (problem, failure) = pool::prepare(&selected, &setting.objective);
    for error in &failure {
        observe(Event::Skip(error));
    }
    if problem.is_empty() {
        return Ok(Count::default());
    }
    let mut network = network(home, setting.guide, &mut observe)?;
    let mut archive: Archive = home.load(home::ARCHIVE)?.unwrap_or_default();
    for entry in &problem {
        renew(&mut archive, entry, &setting.objective);
    }
    home.save(home::ARCHIVE, &archive)?;
    let mut count = Count::default();
    for entry in &problem {
        if setting
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            break;
        }
        count = count.merge(settle(
            entry,
            &mut archive,
            network.as_mut(),
            setting,
            &mut observe,
        ));
        home.save(home::ARCHIVE, &archive)?;
    }
    export::write(home, pool, &archive)?;
    Ok(count)
}
