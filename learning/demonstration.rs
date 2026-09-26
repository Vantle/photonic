use crate::archive::Archive;
use crate::edit::{Action, Bound, apply, legal, seed};
use crate::encoding::{Permutation, encode};
use crate::objective::{Evaluation, Setting, evaluate, potential};
use crate::play::{self, bound};
use crate::problem::Problem;
use crate::task::Task;
use code::atom::Atom;
use code::particle::Particle;
use code::program::Program;
use code::rule::Rule;
use network::input::Sample;
use random::Generator;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::collections::BTreeSet;

const WIDTH: usize = 8;

pub struct Step {
    pub program: Program,
    pub evaluation: Evaluation,
    pub action: Vec<Action>,
    pub chosen: usize,
}

pub struct Demonstration {
    pub step: Vec<Step>,
    pub goal: Evaluation,
}

struct Target<'target> {
    rule: &'target [Rule],
    create: Vec<usize>,
}

fn assign(source: usize, table: &[Vec<usize>], add: &[usize]) -> usize {
    let width = add.len();
    let full = 1usize << width;
    let mut cost = vec![usize::MAX; full];
    cost[0] = 0;
    for row in table.iter().take(source) {
        let mut next = vec![usize::MAX; full];
        for (mask, &known) in cost.iter().enumerate() {
            if known == usize::MAX {
                continue;
            }
            next[mask] = next[mask].min(known + 1);
            for (target, &pair) in row.iter().enumerate() {
                if mask & 1 << target == 0 {
                    let slot = &mut next[mask | 1 << target];
                    *slot = (*slot).min(known + pair);
                }
            }
        }
        cost = next;
    }
    cost.iter()
        .enumerate()
        .filter(|(_, known)| **known != usize::MAX)
        .map(|(mask, known)| {
            known
                + add
                    .iter()
                    .enumerate()
                    .filter(|(target, _)| mask & 1 << target == 0)
                    .map(|(_, cost)| cost)
                    .sum::<usize>()
        })
        .min()
        .unwrap_or(0)
}

fn side(from: &[&Particle], to: &[&Particle]) -> usize {
    let table = from
        .iter()
        .map(|left| to.iter().map(|right| left.difference(right)).collect())
        .collect::<Vec<Vec<usize>>>();
    let add = to
        .iter()
        .map(|particle| particle.len().max(1))
        .collect::<Vec<_>>();
    assign(from.len(), &table, &add)
}

fn output(rule: &Rule) -> Vec<&Particle> {
    crate::coherence::list(rule)
}

fn edit(from: &Rule, to: &Rule) -> usize {
    side(
        &from.input().iter().collect::<Vec<_>>(),
        &to.input().iter().collect::<Vec<_>>(),
    ) + side(&output(from), &output(to))
}

impl<'target> Target<'target> {
    fn new(program: &'target Program, vocabulary: usize, bound: &Bound) -> Option<Self> {
        let reachable = program.rule().iter().all(|rule| {
            rule.input().len() <= bound.particle && rule.output().len() <= bound.particle
        });
        if program.rule().len() > WIDTH || !program.flat() || !reachable {
            return None;
        }
        let create = program
            .rule()
            .iter()
            .map(|rule| {
                1 + (0..vocabulary)
                    .map(|atom| edit(&seed(Atom(atom as u16)), rule))
                    .min()
                    .unwrap_or(0)
            })
            .collect();
        Some(Self {
            rule: program.rule(),
            create,
        })
    }

    fn distance(&self, program: &Program) -> usize {
        let table = program
            .rule()
            .iter()
            .map(|from| self.rule.iter().map(|to| edit(from, to)).collect())
            .collect::<Vec<Vec<usize>>>();
        assign(program.rule().len(), &table, &self.create)
    }
}

pub fn demonstrate(
    task: &Task,
    target: &Program,
    bound: &Bound,
    setting: &Setting,
) -> Option<Demonstration> {
    let vocabulary = task.vocabulary.len();
    let goal = Target::new(target, vocabulary, bound)?;
    let setting = &setting.aim(task.goal);
    let measure = |program: &Program| evaluate(program, &task.example, &task.vocabulary, setting);
    let mut current = Program::default();
    let mut remaining = goal.distance(&current);
    let limit = 2 * remaining + 8;
    let mut step = Vec::new();
    while current != *target {
        if step.len() >= limit {
            return None;
        }
        let action = legal(&current, vocabulary, bound);
        let (chosen, next, distance) = action
            .iter()
            .enumerate()
            .skip(1)
            .filter_map(|(index, &edit)| {
                let next = apply(&current, edit);
                next.flat().then(|| {
                    let distance = goal.distance(&next);
                    (index, next, distance)
                })
            })
            .min_by_key(|(index, _, distance)| (*distance, *index))?;
        if distance >= remaining {
            return None;
        }
        step.push(Step {
            evaluation: measure(&current),
            program: current,
            action,
            chosen,
        });
        current = next;
        remaining = distance;
    }
    let action = legal(&current, vocabulary, bound);
    let evaluation = measure(&current);
    step.push(Step {
        program: current,
        evaluation: evaluation.clone(),
        action,
        chosen: 0,
    });
    Some(Demonstration {
        step,
        goal: evaluation,
    })
}

impl Demonstration {
    pub fn sample(&self, task: &Task, baseline: f64, generator: &mut Generator) -> Vec<Sample> {
        let goal = potential(&self.goal, baseline);
        let mut result = Vec::with_capacity(2 * self.step.len());
        for (index, step) in self.step.iter().enumerate() {
            let permutation = Permutation::new(generator, task.example.len());
            let mut policy = vec![0.0; step.action.len()];
            policy[step.chosen] = 1.0;
            let here = potential(&step.evaluation, baseline);
            result.push(Sample {
                input: encode(
                    task,
                    &step.program,
                    &step.evaluation,
                    &step.action,
                    &permutation,
                ),
                policy,
                value: Some((goal - here) as f32),
                judge: None,
            });
            let Some(next) = self.step.get(index + 1) else {
                continue;
            };
            result.push(Sample {
                input: encode(task, &next.program, &step.evaluation, &[], &permutation),
                policy: Vec::new(),
                value: None,
                judge: Some(potential(&next.evaluation, baseline) as f32),
            });
        }
        result
    }
}

pub struct Lesson {
    demonstration: Vec<Option<Demonstration>>,
    changed: BTreeSet<usize>,
}

fn prepare(problem: &Problem, archive: &Archive, setting: &play::Setting) -> Option<Demonstration> {
    let best = archive.best(&problem.task.name)?;
    demonstrate(
        &problem.task,
        &best.program,
        &bound(&setting.bound, problem),
        &setting.objective,
    )
}

impl Lesson {
    pub fn new(problem: &[Problem], archive: &Archive, setting: &play::Setting) -> Self {
        Self {
            demonstration: problem
                .into_par_iter()
                .map(|problem| prepare(problem, archive, setting))
                .collect(),
            changed: BTreeSet::new(),
        }
    }

    pub fn change(&mut self, index: usize) {
        self.changed.insert(index);
    }

    pub fn refresh(&mut self, problem: &[Problem], archive: &Archive, setting: &play::Setting) {
        let changed = std::mem::take(&mut self.changed)
            .into_iter()
            .collect::<Vec<_>>();
        let fresh = changed
            .clone()
            .into_par_iter()
            .map(|index| prepare(&problem[index], archive, setting))
            .collect::<Vec<_>>();
        for (index, demonstration) in changed.into_iter().zip(fresh) {
            self.demonstration[index] = demonstration;
        }
    }

    pub fn count(&self) -> usize {
        self.demonstration.iter().flatten().count()
    }

    pub fn sample(&self, problem: &[Problem], generator: &mut Generator) -> Vec<Sample> {
        self.demonstration
            .iter()
            .zip(problem)
            .filter_map(|(demonstration, problem)| Some((demonstration.as_ref()?, problem)))
            .flat_map(|(demonstration, problem)| {
                demonstration.sample(&problem.task, problem.baseline, generator)
            })
            .collect()
    }
}
