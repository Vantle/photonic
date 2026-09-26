use crate::argument::Solve;
use crate::output::{line, proof, report};
use crate::setup::{open, read};
use learning::attempt;
use learning::edit::Bound;
use learning::home;
use learning::import;
use learning::objective;
use learning::pool;
use learning::solution::Budget;
use learning::task::{self, Goal, Task};
use miette::{IntoDiagnostic, miette};
use std::time::{Duration, Instant};

fn aimed(argument: &Solve) -> bool {
    argument.processor.is_some() || argument.size.is_some()
}

fn goal(argument: &Solve, fallback: Goal) -> Result<Goal, task::Failure> {
    Goal::new(
        argument.processor.unwrap_or(fallback.processor()),
        argument.size.unwrap_or(fallback.size()),
    )
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
    let goal = if aimed(argument) {
        Some(goal(argument, Goal::default()).into_diagnostic()?)
    } else {
        None
    };
    Ok(Some(Task { goal, ..task }))
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
            let task = task::find(&pool, name).into_diagnostic()?;
            let goal = goal(argument, task.goal.unwrap_or_default()).into_diagnostic()?;
            variant.push(Task {
                name: format!("{name}.p{}.s{}", goal.processor(), goal.size()),
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
    let count = attempt::run(
        &home,
        &pool,
        &chosen,
        &attempt::Setting {
            objective,
            bound: Bound::default(),
            budget: Budget {
                size: argument.limit,
                time: argument.enumerate.map(Duration::from_secs),
            },
            guide: Duration::from_secs(argument.guide),
            blind: argument.blind,
            deadline: None,
        },
        |event| report(event, argument.show),
    )
    .into_diagnostic()?;
    line(&format!(
        "found programs for {} of {} tasks, {}, {} passing their held-out tests, in {:.1}s",
        count.found,
        count.total,
        proof(&count),
        count.general,
        start.elapsed().as_secs_f64()
    ));
    Ok(())
}
