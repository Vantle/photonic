use crate::corpus::curated;
use crate::encoding::ATOM;
use crate::objective::Setting;
use crate::problem::{self, Problem};
use crate::synthetic::generate;
use crate::task::{Goal, Task};
use code::hashing::combine;
use random::Generator;

pub(crate) const HIDDEN: usize = 4;
pub const SYNTHETIC: usize = 48;
const PROCESSOR: [f64; 3] = [1.0, 4.0, 64.0];
const SIZE: [f64; 3] = [0.0125, 0.05, 0.2];

fn vary(task: Task, generator: &mut Generator) -> Task {
    if generator.chance(0.5) {
        return task;
    }
    let processor = PROCESSOR[generator.below(PROCESSOR.len())];
    let size = SIZE[generator.below(SIZE.len())];
    Task {
        goal: Goal::new(processor, size).ok(),
        ..task
    }
}

pub(crate) fn conceal(task: Task) -> Result<Task, problem::Failure> {
    let name = task.name.clone();
    task.conceal(HIDDEN)
        .map_err(|source| problem::Failure::Hidden { task: name, source })
}

pub fn initial(
    synthetic: usize,
    seed: u64,
    setting: &Setting,
) -> Result<Vec<Task>, problem::Failure> {
    let mut generator = Generator::new(seed);
    curated()
        .into_iter()
        .chain((0..synthetic).map(|index| {
            let rule = 2 + generator.below(3);
            let task = generate(
                &mut generator,
                format!("synthetic.{index}"),
                &setting.limit,
                rule,
            );
            vary(task, &mut generator)
        }))
        .map(conceal)
        .collect()
}

pub fn merge(mut pool: Vec<Task>) -> Result<Vec<Task>, problem::Failure> {
    for task in curated() {
        if pool.iter().all(|known| known.name != task.name) {
            pool.push(conceal(task)?);
        }
    }
    Ok(pool)
}

pub fn grow(
    mut pool: Vec<Task>,
    count: usize,
    seed: u64,
    setting: &Setting,
) -> Result<Vec<Task>, problem::Failure> {
    let mut generator = Generator::new(combine(seed, pool.len() as u64));
    let start = pool
        .iter()
        .filter_map(|task| task.name.strip_prefix("synthetic."))
        .filter_map(|suffix| suffix.parse::<usize>().ok())
        .max()
        .map_or(0, |maximum| maximum + 1);
    for index in start..start + count {
        let rule = 2 + generator.below(3);
        let task = generate(
            &mut generator,
            format!("synthetic.{index}"),
            &setting.limit,
            rule,
        );
        pool.push(conceal(vary(task, &mut generator))?);
    }
    Ok(pool)
}

fn pose(task: &Task, setting: &Setting) -> Result<Problem, problem::Failure> {
    Problem::new(task.clone(), setting, ATOM)
}

pub fn admit(
    pool: Vec<Task>,
    task: Task,
    setting: &Setting,
) -> Result<Vec<Task>, problem::Failure> {
    let task = conceal(task)?;
    pose(&task, setting)?;
    let mut pool = pool
        .into_iter()
        .filter(|known| known.name != task.name)
        .collect::<Vec<_>>();
    pool.push(task);
    Ok(pool)
}

pub fn prepare(pool: &[Task], setting: &Setting) -> (Vec<Problem>, Vec<problem::Failure>) {
    let mut ready = Vec::new();
    let mut failure = Vec::new();
    for task in pool {
        match pose(task, setting) {
            Ok(problem) => ready.push(problem),
            Err(error) => failure.push(error),
        }
    }
    (ready, failure)
}
