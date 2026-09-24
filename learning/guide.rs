use crate::edit::{Action, Bound, apply, legal};
use crate::encoding::{Permutation, Shape, encode};
use crate::objective::{Evaluation, Setting, evaluate};
use crate::task::Task;
use code::program::Program;
use gpu::engine::Engine;
use network::checkpoint;
use network::configuration::Configuration;
use network::grow::grow;
use network::input::{Input, Output};
use network::model::Model;
use random::Generator;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};
use std::path::Path;
use std::time::{Duration, Instant};
use thiserror::Error;

const BREADTH: usize = 24;
const MIX: f64 = 0.05;
const BATCH: usize = 64;

#[derive(Debug, Error)]
pub enum Failure {
    #[error(transparent)]
    Checkpoint(#[from] checkpoint::Failure),
    #[error(
        "the saved network reads an older encoding; one training run grows it to the current one"
    )]
    Interface,
}

pub struct Network {
    model: Model,
    engine: Option<Engine>,
}

impl Network {
    pub fn load(path: &Path) -> Result<Self, Failure> {
        let (saved, _) = checkpoint::load(path)?;
        let current = Shape::default().architecture();
        let target = Configuration {
            field: current.field,
            unary: current.unary,
            binary: current.binary,
            judge: current.judge,
            ..saved.configuration().clone()
        };
        let model = if target == *saved.configuration() {
            saved
        } else {
            grow(&saved, target, &mut Generator::new(0)).map_err(|_| Failure::Interface)?
        };
        let engine = Engine::new(&model).ok();
        Ok(Self { model, engine })
    }

    pub fn infer(&mut self, input: &[&Input]) -> Vec<Output> {
        self.engine
            .as_mut()
            .and_then(|engine| engine.infer(input).ok())
            .unwrap_or_else(|| self.model.infer(input))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Effort {
    pub time: Duration,
    pub expansion: u64,
}

pub struct Guidance {
    pub expanded: u64,
    pub first: Option<u64>,
    pub best: Option<(Program, Evaluation)>,
}

struct Node {
    program: Program,
    depth: u32,
    chance: f64,
}

struct Entry {
    cost: f64,
    parent: Option<usize>,
    action: Action,
    depth: u32,
    chance: f64,
}

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.cost.total_cmp(&other.cost) == Ordering::Equal
    }
}

impl Eq for Entry {}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Entry {
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.total_cmp(&self.cost)
    }
}

fn probability(logit: &[f32]) -> Vec<f64> {
    let maximum = logit.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let exponent = logit
        .iter()
        .map(|&value| f64::from(value - maximum).exp())
        .collect::<Vec<_>>();
    let total = exponent.iter().sum::<f64>();
    let uniform = MIX / logit.len().max(1) as f64;
    exponent
        .into_iter()
        .map(|value| (1.0 - MIX) * value / total + uniform)
        .collect()
}

pub fn search(
    task: &Task,
    bound: &Bound,
    setting: &Setting,
    effort: Effort,
    goal: f64,
    network: &mut Network,
) -> Guidance {
    let deadline = Instant::now() + effort.time;
    let setting = &setting.aim(task.goal);
    let mut generator = Generator::new(task.example.len() as u64);
    let vocabulary = task.vocabulary.len();
    let mut arena: Vec<Node> = Vec::new();
    let mut seen: HashSet<Program> = HashSet::new();
    let mut frontier = BinaryHeap::from([Entry {
        cost: 0.0,
        parent: None,
        action: Action::Stop,
        depth: 0,
        chance: 0.0,
    }]);
    let mut guidance = Guidance {
        expanded: 0,
        first: None,
        best: None,
    };
    while Instant::now() < deadline
        && guidance.expanded < effort.expansion
        && guidance
            .best
            .as_ref()
            .is_none_or(|(_, best)| best.cost > goal + 1e-9)
    {
        let mut batch = Vec::with_capacity(BATCH);
        while batch.len() < BATCH {
            let Some(entry) = frontier.pop() else {
                break;
            };
            let program = entry.parent.map_or_else(Program::default, |parent| {
                apply(&arena[parent].program, entry.action)
            });
            if seen.insert(program.clone()) {
                batch.push(Node {
                    program,
                    depth: entry.depth,
                    chance: entry.chance,
                });
            }
        }
        if batch.is_empty() {
            break;
        }
        let evaluation = batch
            .par_iter()
            .map(|node| evaluate(&node.program, &task.example, &task.vocabulary, setting))
            .collect::<Vec<_>>();
        for (node, evaluation) in batch.iter().zip(&evaluation) {
            guidance.expanded += 1;
            let better = guidance
                .best
                .as_ref()
                .is_none_or(|(_, best)| evaluation.cost < best.cost - 1e-9);
            if evaluation.correct && better {
                guidance.first.get_or_insert(guidance.expanded);
                guidance.best = Some((node.program.clone(), evaluation.clone()));
            }
        }
        let action = batch
            .iter()
            .map(|node| legal(&node.program, vocabulary, bound))
            .collect::<Vec<_>>();
        let input = batch
            .iter()
            .zip(&evaluation)
            .zip(&action)
            .map(|((node, evaluation), action)| {
                let permutation = Permutation::new(&mut generator, task.example.len());
                encode(task, &node.program, evaluation, action, &permutation)
            })
            .collect::<Vec<_>>();
        let output = network.infer(&input.iter().collect::<Vec<_>>());
        for ((node, action), output) in batch.into_iter().zip(action).zip(output) {
            let parent = arena.len();
            let chance = probability(&output.logit);
            let mut ranked = (1..action.len()).collect::<Vec<_>>();
            ranked.sort_by(|&left, &right| chance[right].total_cmp(&chance[left]));
            for &choice in ranked.iter().take(BREADTH) {
                let depth = node.depth + 1;
                let path = node.chance + chance[choice].ln();
                frontier.push(Entry {
                    cost: f64::from(depth).ln() - path,
                    parent: Some(parent),
                    action: action[choice],
                    depth,
                    chance: path,
                });
            }
            arena.push(node);
        }
    }
    guidance
}
