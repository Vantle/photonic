use crate::argument::Solve;
use crate::output::{discovery, line};
use crate::setup::{export, open, read};
use learning::archive::{Archive, Record, moment};
use learning::edit::Bound;
use learning::export::source;
use learning::guide::{self, Effort, Guidance, Network};
use learning::home::{self, Home};
use learning::import;
use learning::objective;
use learning::play;
use learning::pool;
use learning::problem::Problem;
use learning::solution::{self, Budget, Solution};
use learning::task::{self, Task};
use miette::{IntoDiagnostic, miette};
use std::time::{Duration, Instant};

fn goal(argument: &Solve) -> Option<task::Goal> {
    let known = task::Goal::default();
    (argument.processor.is_some() || argument.size.is_some()).then(|| task::Goal {
        processor: argument.processor.unwrap_or(known.processor),
        size: argument.size.unwrap_or(known.size),
    })
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
        goal: goal(argument),
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
    let found = solution.cost <= known + 1e-9;
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

pub fn attempt(
    home: &Home,
    pool: &[Task],
    chosen: &[String],
    option: &Attempt,
) -> miette::Result<Count> {
    let setting = objective::Setting::default();
    let checkpoint = home.file(home::CHECKPOINT);
    let mut network = (option.guide > 0 && checkpoint.exists())
        .then(|| Network::load(&checkpoint))
        .transpose()
        .into_diagnostic()?;
    let (problem, failure) = pool::prepare(pool, &setting);
    for error in failure {
        line(&format!("skipped: {error}"));
    }
    if let Some(name) = chosen
        .iter()
        .find(|name| problem.iter().all(|entry| entry.task.name != **name))
    {
        return Err(miette!("no usable task is named {name}"));
    }
    let mut archive: Archive = home
        .load(home::ARCHIVE)
        .into_diagnostic()?
        .unwrap_or_default();
    let mut count = Count::default();
    for entry in problem
        .iter()
        .filter(|entry| chosen.is_empty() || chosen.contains(&entry.task.name))
    {
        let task = &entry.task;
        let known = if option.blind {
            f64::INFINITY
        } else {
            incumbent(entry, &archive, &setting)
        };
        let begun = Instant::now();
        let solution = solution::solve(task, known, &setting, option.budget).into_diagnostic()?;
        line(&describe(task, known, &solution, begun.elapsed()));
        let mut candidate = solution.optimal.clone();
        let mut proven = solution.proven && solution.cost <= known + 1e-9;
        if let Some(network) = network.as_mut()
            && !solution.proven
        {
            let gap = solution.gap();
            let guidance = guide::search(
                task,
                &play::bound(&Bound::default(), entry),
                &setting,
                Effort {
                    time: Duration::from_secs(option.guide),
                    expansion: u64::MAX,
                },
                gap,
                network,
            );
            line(&guided(task, &guidance));
            if let Some((_, evaluation)) = &guidance.best
                && evaluation.cost <= gap + 1e-9
                && evaluation.cost <= known + 1e-9
            {
                proven = true;
                line(&format!(
                    "{}: optimal, because every program above size {} costs at least {gap:.3}",
                    task.name,
                    solution.size.unwrap_or(0)
                ));
            }
            candidate.extend(guidance.best);
        }
        count.total += 1;
        count.found += usize::from(!candidate.is_empty());
        count.proven += usize::from(proven);
        for (program, evaluation) in solution.optimal.iter().take(option.show) {
            line(&format!(
                "size {}, work {:.2}, span {:.2}\n{}",
                evaluation.size.total(),
                evaluation.work,
                evaluation.span,
                source(task, program)
            ));
        }
        archive.register(&task.name, entry.baseline, None);
        let least = candidate
            .iter()
            .map(|(_, evaluation)| evaluation.cost)
            .fold(f64::INFINITY, f64::min);
        let mut held = false;
        for (program, evaluation) in &candidate {
            let passes = task.holdout.is_empty()
                || objective::evaluate(program, &task.holdout, &task.vocabulary, &setting).correct;
            held |= passes;
            let record = Record::new(program.clone(), evaluation, passes, moment());
            let record = if proven && evaluation.cost <= least + 1e-9 {
                record.prove()
            } else {
                record
            };
            if let Some(improvement) = archive.offer(&task.name, record) {
                line(&discovery(&improvement));
            }
        }
        count.general += usize::from(held);
        if !task.holdout.is_empty() && !candidate.is_empty() {
            line(&format!(
                "{}: {} {} held-out tests",
                task.name,
                if held { "passes" } else { "fails" },
                task.holdout.len()
            ));
        }
    }
    home.save(home::ARCHIVE, &archive).into_diagnostic()?;
    export(home, pool)?;
    Ok(count)
}

pub fn run(argument: &Solve) -> miette::Result<()> {
    let home = open(&argument.home)?;
    let mut pool = home
        .load::<Vec<Task>>(home::POOL)
        .into_diagnostic()?
        .unwrap_or_default();
    let mut chosen = argument.task.clone();
    if let Some(task) = tested(argument)? {
        chosen.push(task.name.clone());
        pool = pool::admit(pool, task);
        home.save(home::POOL, &pool).into_diagnostic()?;
    } else if let Some(goal) = goal(argument) {
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
    let start = Instant::now();
    let count = attempt(
        &home,
        &pool,
        &chosen,
        &Attempt {
            budget: Budget {
                size: argument.limit,
                time: argument.budget.map(Duration::from_secs),
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
