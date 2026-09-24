use crate::edit::Bound;
use crate::guide::{Effort, Network, search};
use crate::objective::Setting;
use crate::pool::HIDDEN;
use crate::solution::{Budget, solve};
use crate::synthetic::generate;
use crate::task::Task;
use random::Generator;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Exam {
    pub task: Task,
    pub cost: f64,
    pub size: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Mark {
    pub round: usize,
    pub level: usize,
    pub mastery: Vec<f64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Curriculum {
    pub level: usize,
    pub bred: usize,
    pub exam: Vec<Vec<Exam>>,
    pub mark: Vec<Mark>,
}

impl Default for Curriculum {
    fn default() -> Self {
        Self {
            level: 1,
            bred: 0,
            exam: Vec::new(),
            mark: Vec::new(),
        }
    }
}

pub fn prefix(level: usize) -> String {
    format!("level{level}.")
}

impl Curriculum {
    pub fn breed(&mut self, level: usize, seed: u64, setting: &Setting) -> Task {
        let mut generator = Generator::new(seed ^ (self.bred as u64 + 1).wrapping_mul(0x9e37_79b9));
        let name = format!("{}{}", prefix(level), self.bred);
        self.bred += 1;
        let task = generate(&mut generator, name, &setting.limit, level);
        Task {
            reference: None,
            ..task
        }
        .conceal(HIDDEN)
    }

    pub fn exam(&mut self, level: usize) -> &mut Vec<Exam> {
        while self.exam.len() < level {
            self.exam.push(Vec::new());
        }
        &mut self.exam[level - 1]
    }
}

pub fn grade(task: Task, setting: &Setting, time: Duration) -> Option<Exam> {
    let budget = Budget {
        size: 16,
        time: Some(time),
    };
    let solution = solve(&task, f64::INFINITY, setting, budget).ok()?;
    if !solution.proven {
        return None;
    }
    let (_, evaluation) = solution.optimal.first()?;
    Some(Exam {
        cost: solution.cost,
        size: evaluation.size.total(),
        task,
    })
}

pub fn examine(exam: &[Exam], network: &mut Network, setting: &Setting, expansion: u64) -> f64 {
    if exam.is_empty() {
        return 0.0;
    }
    let passed = exam
        .iter()
        .filter(|exam| {
            let guidance = search(
                &exam.task,
                &Bound::default(),
                setting,
                Effort {
                    time: Duration::from_secs(120),
                    expansion,
                },
                exam.cost,
                network,
            );
            guidance
                .best
                .as_ref()
                .is_some_and(|(_, evaluation)| evaluation.cost <= exam.cost + 1e-9)
        })
        .count();
    passed as f64 / exam.len() as f64
}

pub fn focus(pool: &[Task], level: usize, rehearse: f64, seed: u64) -> (usize, Vec<String>) {
    let named = |grade: usize| {
        pool.iter()
            .filter(move |task| task.name.starts_with(&prefix(grade)))
            .map(|task| task.name.clone())
    };
    let current = named(level).collect::<Vec<_>>();
    let mut earlier = (1..level).flat_map(named).collect::<Vec<_>>();
    Generator::new(seed).shuffle(&mut earlier);
    let share =
        (rehearse * current.len() as f64 / (1.0 - rehearse).max(f64::EPSILON)).ceil() as usize;
    let count = current.len();
    (
        count,
        current
            .into_iter()
            .chain(earlier.into_iter().take(share))
            .collect(),
    )
}
