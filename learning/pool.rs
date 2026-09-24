use crate::corpus::curated;
use crate::encoding::ATOM;
use crate::objective::Setting;
use crate::problem::{self, Problem};
use crate::synthetic::generate;
use crate::task::{Goal, Task};
use random::Generator;

pub const HIDDEN: usize = 4;
const PROCESSOR: [f64; 3] = [1.0, 4.0, 64.0];
const SIZE: [f64; 3] = [0.0125, 0.05, 0.2];

fn aim(task: Task, generator: &mut Generator) -> Task {
    if generator.chance(0.5) {
        return task;
    }
    Task {
        goal: Some(Goal {
            processor: PROCESSOR[generator.below(PROCESSOR.len())],
            size: SIZE[generator.below(SIZE.len())],
        }),
        ..task
    }
}

pub fn initial(synthetic: usize, seed: u64, setting: &Setting) -> Vec<Task> {
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
            aim(task, &mut generator)
        }))
        .map(|task| task.conceal(HIDDEN))
        .collect()
}

pub fn merge(mut pool: Vec<Task>) -> Vec<Task> {
    for task in curated() {
        if pool.iter().all(|known| known.name != task.name) {
            pool.push(task.conceal(HIDDEN));
        }
    }
    pool
}

pub fn grow(mut pool: Vec<Task>, count: usize, seed: u64, setting: &Setting) -> Vec<Task> {
    let mut generator = Generator::new(seed ^ pool.len() as u64);
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
        pool.push(aim(task, &mut generator).conceal(HIDDEN));
    }
    pool
}

pub fn admit(pool: Vec<Task>, task: Task) -> Vec<Task> {
    let mut pool = pool
        .into_iter()
        .filter(|known| known.name != task.name)
        .collect::<Vec<_>>();
    pool.push(task.conceal(HIDDEN));
    pool
}

pub fn prepare(pool: &[Task], setting: &Setting) -> (Vec<Problem>, Vec<problem::Failure>) {
    let mut ready = Vec::new();
    let mut failure = Vec::new();
    let thorough = setting.thorough();
    for task in pool {
        match Problem::new(task.clone(), &thorough, ATOM) {
            Ok(problem) => ready.push(problem),
            Err(error) => failure.push(error),
        }
    }
    (ready, failure)
}
