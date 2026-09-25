use crate::argument::Solve;
use crate::output::{discovery, line};
use crate::setup::{archive, export, open, read};
use code::program::Program;
use learning::archive::{Archive, Record, moment};
use learning::edit::Bound;
use learning::export::source;
use learning::guide::{self, Effort, Guidance, Network};
use learning::home::{self, Home};
use learning::import;
use learning::objective::{self, TOLERANCE};
use learning::play;
use learning::pool;
use learning::problem::Problem;
use learning::renewal::renew;
use learning::solution::{self, Budget, Solution};
use learning::task::{Goal, Task};
use miette::{IntoDiagnostic, miette};
use std::time::{Duration, Instant};

fn aimed(argument: &Solve) -> bool {
    argument.processor.is_some() || argument.size.is_some()
}

fn goal(argument: &Solve, fallback: Goal) -> Goal {
    Goal {
        processor: argument.processor.unwrap_or(fallback.processor),
        size: argument.size.unwrap_or(fallback.size),
    }
}

fn tested(argument: &Solve) -> miette::Result<Option<Task>> {
    if argument.input.is_empty() && argument.output.is_empty() {
        return Ok(None);
    }
    if argument.input.len() != argument.output.len() {
        return Err(miette!("pass one --output for every --input"));
    }
    let pair = argument
        .input
        .iter()
        .zip(&argument.output)
        .map(|(input, output)| Ok((read(input)?, read(output)?)))
        .collect::<miette::Result<Vec<_>>>()?;
    let name = argument.name.clone().unwrap_or_else(|| "tests".to_owned());
    let task = import::define(&name, &pair).into_diagnostic()?;
    Ok(Some(Task {
        goal: aimed(argument).then(|| goal(argument, Goal::default())),
        ..task
    }))
}

fn incumbent(problem: &Problem, archive: &Archive, setting: &objective::Setting) -> f64 {
    let task = &problem.task;
    archive
        .entry(&task.name)
        .and_then(|entry| entry.best.as_ref())
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

fn describe(task: &Task, known: f64, solution: &Solution, elapsed: Duration) -> String {
    let examined = format!(
        "{} programs examined in {:.1}s",
        solution.examined,
        elapsed.as_secs_f64()
    );
    let reached = solution.size.map_or_else(
        || "before any size was complete".to_owned(),
        |size| format!("every program up to size {size}"),
    );
    let found = solution.cost <= known + TOLERANCE;
    match (solution.proven, found) {
        (true, true) if solution.complete => format!(
            "{}: optimal cost {:.3}, reached by {} of {examined}",
            task.name,
            solution.cost,
            solution.optimal.len()
        ),
        (true, true) => format!(
            "{}: optimal cost {:.3}, reached by {} of {examined}; programs above size {} may tie it",
            task.name,
            solution.cost,
            solution.optimal.len(),
            solution.size.unwrap_or(0)
        ),
        (true, false) if solution.complete => format!(
            "{}: no flat program is as cheap as the known {known:.3}; {examined}",
            task.name
        ),
        (true, false) => format!(
            "{}: no flat program is cheaper than the known {known:.3}; programs above size {} may tie it; {examined}",
            task.name,
            solution.size.unwrap_or(0)
        ),
        (false, _) if solution.cost.is_finite() => format!(
            "{}: best cost {:.3} after {reached}; a cheaper program costs at least {:.3}; {examined}",
            task.name,
            solution.cost,
            solution.gap()
        ),
        (false, _) => format!("{}: nothing correct in {reached}; {examined}", task.name),
    }
}

fn guided(task: &Task, guidance: &Guidance) -> String {
    match (&guidance.best, guidance.first) {
        (Some((_, evaluation)), Some(first)) => format!(
            "{}: guided search found cost {:.3} at size {} among {} programs, the first correct one after {first}",
            task.name,
            evaluation.cost,
            evaluation.size.total(),
            guidance.expanded
        ),
        _ => format!(
            "{}: guided search found nothing correct among {} programs",
            task.name, guidance.expanded
        ),
    }
}

pub struct Attempt {
    pub objective: objective::Setting,
    pub bound: Bound,
    pub budget: Budget,
    pub guide: u64,
    pub blind: bool,
    pub show: usize,
}

#[derive(Default)]
pub struct Count {
    pub total: usize,
    pub found: usize,
    pub proven: usize,
    pub general: usize,
}

fn network(home: &Home, guide: u64) -> miette::Result<Option<Network>> {
    if guide == 0 {
        return Ok(None);
    }
    let checkpoint = home.file(home::CHECKPOINT);
    if !checkpoint.exists() {
        line(&format!(
            "guided search skipped: no trained network is saved in {} yet",
            home.path().display()
        ));
        return Ok(None);
    }
    Network::load(&checkpoint).map(Some).into_diagnostic()
}

struct Finding {
    candidate: Vec<(Program, objective::Evaluation)>,
    proven: bool,
}

struct Cover {
    size: Option<usize>,
    gap: f64,
}

impl Cover {
    fn reason(&self) -> String {
        match self.size {
            Some(size) => format!(
                "every program above size {size} costs at least {:.3}",
                self.gap
            ),
            None => format!("every correct program costs at least {:.3}", self.gap),
        }
    }
}

fn exhaust(task: &Task, known: f64, option: &Attempt) -> (Finding, Option<Cover>) {
    let begun = Instant::now();
    let solution = match solution::solve(task, known, &option.objective, option.budget) {
        Ok(solution) => solution,
        Err(failure) => {
            line(&format!("{}: {failure}", task.name));
            let finding = Finding {
                candidate: Vec::new(),
                proven: false,
            };
            let cover = Cover {
                size: None,
                gap: solution::least(task, &option.objective),
            };
            return (finding, Some(cover));
        }
    };
    line(&describe(task, known, &solution, begun.elapsed()));
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

fn find(entry: &Problem, known: f64, network: Option<&mut Network>, option: &Attempt) -> Finding {
    let task = &entry.task;
    let (mut finding, cover) = exhaust(task, known, option);
    if let (Some(network), Some(cover)) = (network, cover) {
        let guidance = guide::search(
            task,
            &play::bound(&option.bound, entry),
            &option.objective,
            Effort {
                time: Duration::from_secs(option.guide),
                expansion: u64::MAX,
            },
            cover.gap,
            network,
        );
        line(&guided(task, &guidance));
        if let Some((_, evaluation)) = &guidance.best
            && evaluation.cost <= cover.gap + TOLERANCE
            && evaluation.cost <= known + TOLERANCE
        {
            finding.proven = true;
            line(&format!(
                "{}: optimal, because {}",
                task.name,
                cover.reason()
            ));
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

fn keep(entry: &Problem, finding: &Finding, archive: &mut Archive, option: &Attempt) -> bool {
    let task = &entry.task;
    let goal = option.objective.aim(task.goal).goal;
    for (program, evaluation) in finding.candidate.iter().take(option.show) {
        line(&format!(
            "size {}, work {:.2}, span {:.2}\n{}",
            evaluation.size.total(),
            evaluation.work,
            evaluation.span,
            source(task, program)
        ));
    }
    let least = finding
        .candidate
        .first()
        .map_or(f64::INFINITY, |(_, evaluation)| evaluation.cost);
    let mut held = false;
    for (program, evaluation) in &finding.candidate {
        let passes = objective::general(program, task, &option.objective);
        held |= passes;
        let record = Record::new(program.clone(), evaluation, passes, moment());
        let record = if finding.proven && evaluation.cost <= least + TOLERANCE {
            record.prove(goal)
        } else {
            record
        };
        if let Some(improvement) = archive.offer(&task.name, record) {
            line(&discovery(&improvement));
        }
    }
    if !task.holdout.is_empty() && !finding.candidate.is_empty() {
        line(&format!(
            "{}: {} {} held-out tests",
            task.name,
            if held { "passes" } else { "fails" },
            task.holdout.len()
        ));
    }
    held
}

fn settle(
    entry: &Problem,
    archive: &mut Archive,
    network: Option<&mut Network>,
    option: &Attempt,
    count: &mut Count,
) {
    count.total += 1;
    let known = if option.blind {
        f64::INFINITY
    } else {
        incumbent(entry, archive, &option.objective)
    };
    let finding = find(entry, known, network, option);
    let held = keep(entry, &finding, archive, option);
    count.found += usize::from(!finding.candidate.is_empty());
    count.proven += usize::from(finding.proven);
    count.general += usize::from(held);
}

pub fn attempt(
    home: &Home,
    pool: &[Task],
    chosen: &[String],
    option: &Attempt,
) -> miette::Result<Count> {
    if let Some(name) = chosen
        .iter()
        .find(|name| pool.iter().all(|task| task.name != **name))
    {
        return Err(miette!("no task is named {name}"));
    }
    let selected = pool
        .iter()
        .filter(|task| chosen.contains(&task.name))
        .cloned()
        .collect::<Vec<_>>();
    let (problem, failure) = pool::prepare(&selected, &option.objective);
    for error in failure {
        line(&format!("skipped: {error}"));
    }
    if problem.is_empty() {
        return Ok(Count::default());
    }
    let mut network = network(home, option.guide)?;
    let mut archive = archive(home)?;
    for entry in &problem {
        renew(&mut archive, entry, &option.objective);
    }
    home.save(home::ARCHIVE, &archive).into_diagnostic()?;
    let mut count = Count::default();
    for entry in &problem {
        settle(entry, &mut archive, network.as_mut(), option, &mut count);
        home.save(home::ARCHIVE, &archive).into_diagnostic()?;
    }
    export(home, pool)?;
    Ok(count)
}

pub fn run(argument: &Solve) -> miette::Result<()> {
    let home = open(&argument.home)?;
    let objective = objective::Setting::default();
    let mut pool = home
        .load::<Vec<Task>>(home::POOL)
        .into_diagnostic()?
        .unwrap_or_default();
    let mut chosen = argument.task.clone();
    if let Some(task) = tested(argument)? {
        chosen.push(task.name.clone());
        pool = pool::admit(pool, task, &objective).into_diagnostic()?;
        home.save(home::POOL, &pool).into_diagnostic()?;
    } else if aimed(argument) {
        if chosen.is_empty() {
            return Err(miette!(
                "name the tasks to solve under --processor or --size with --task"
            ));
        }
        let mut variant = Vec::new();
        for name in &chosen {
            let task = pool
                .iter()
                .find(|task| task.name == *name)
                .ok_or_else(|| miette!("no task is named {name}"))?;
            let goal = goal(argument, task.goal.unwrap_or_default());
            variant.push(Task {
                name: format!("{name}.p{}.s{}", goal.processor, goal.size),
                goal: Some(goal),
                ..task.clone()
            });
        }
        chosen = variant.iter().map(|task| task.name.clone()).collect();
        pool.retain(|task| !chosen.contains(&task.name));
        pool.extend(variant);
        home.save(home::POOL, &pool).into_diagnostic()?;
    }
    if chosen.is_empty() {
        chosen = pool.iter().map(|task| task.name.clone()).collect();
    }
    let start = Instant::now();
    let count = attempt(
        &home,
        &pool,
        &chosen,
        &Attempt {
            objective,
            bound: Bound::default(),
            budget: Budget {
                size: argument.limit,
                time: argument.enumerate.map(Duration::from_secs),
            },
            guide: argument.guide,
            blind: argument.blind,
            show: argument.show,
        },
    )?;
    line(&format!(
        "found programs for {} of {} tasks, {} proven optimal, {} passing their held-out tests, in {:.1}s",
        count.found,
        count.total,
        count.proven,
        count.general,
        start.elapsed().as_secs_f64()
    ));
    Ok(())
}
