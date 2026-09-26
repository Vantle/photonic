use crate::edit::Action;
use crate::objective::Evaluation;
use code::program::Program;
use random::Generator;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Setting {
    pub simulation: usize,
    pub considered: usize,
    pub visit: f64,
    pub scale: f64,
    pub step: usize,
}

impl Default for Setting {
    fn default() -> Self {
        Self {
            simulation: 16,
            considered: 16,
            visit: 50.0,
            scale: 0.1,
            step: 24,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Position {
    pub program: Arc<Program>,
    pub evaluation: Option<Arc<Evaluation>>,
    pub potential: f64,
}

pub(crate) trait Environment {
    fn transition(&mut self, position: &Position, action: Action) -> Position;
    fn verify(&mut self, parent: &Position, position: &Position) -> Position;
    fn legal(&mut self, position: &Position) -> Vec<Action>;
}

struct Node {
    position: Position,
    reward: f64,
    depth: usize,
    terminal: bool,
    action: Vec<Action>,
    logit: Vec<f32>,
    raw: f64,
    child: Vec<Option<usize>>,
    visit: Vec<u32>,
    sum: f64,
    count: u32,
}

impl Node {
    fn new(
        position: Position,
        reward: f64,
        depth: usize,
        terminal: bool,
        action: Vec<Action>,
    ) -> Self {
        let width = action.len();
        Self {
            position,
            reward,
            depth,
            terminal,
            action,
            logit: vec![0.0; width],
            raw: 0.0,
            child: vec![None; width],
            visit: vec![0; width],
            sum: 0.0,
            count: 0,
        }
    }

    fn value(&self) -> f64 {
        if self.count == 0 {
            return 0.0;
        }
        self.sum / f64::from(self.count)
    }
}

pub(crate) struct Leaf {
    node: usize,
    path: Vec<(usize, usize)>,
}

pub(crate) struct Tree {
    node: Vec<Node>,
    gumbel: Vec<f64>,
    sequence: Vec<usize>,
    setting: Setting,
    simulation: usize,
}

pub(crate) fn sequence(considered: usize, simulation: usize) -> Vec<usize> {
    if considered <= 1 {
        return (0..simulation).collect();
    }
    let phase = (considered as f64).log2().ceil() as usize;
    let mut result = Vec::with_capacity(simulation);
    let mut visit = vec![0; considered];
    let mut remaining = considered;
    while result.len() < simulation {
        let extra = (simulation / (phase * remaining)).max(1);
        for _ in 0..extra {
            result.extend_from_slice(&visit[..remaining]);
            for count in &mut visit[..remaining] {
                *count += 1;
            }
        }
        remaining = (remaining / 2).max(2);
    }
    result.truncate(simulation);
    result
}

fn softmax(logit: &[f64]) -> Vec<f64> {
    let maximum = logit.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let exponent = logit
        .iter()
        .map(|value| (value - maximum).exp())
        .collect::<Vec<_>>();
    let total = exponent.iter().sum::<f64>();
    exponent.into_iter().map(|value| value / total).collect()
}

fn best(score: impl Iterator<Item = (usize, f64)>) -> Option<usize> {
    score
        .filter(|(_, value)| value.is_finite())
        .fold(
            None,
            |leader: Option<(usize, f64)>, (index, value)| match leader {
                Some((_, known)) if known >= value => leader,
                _ => Some((index, value)),
            },
        )
        .map(|(index, _)| index)
}

impl Tree {
    pub fn new(position: Position, depth: usize, action: Vec<Action>, setting: Setting) -> Self {
        Self {
            node: vec![Node::new(position, 0.0, depth, false, action)],
            gumbel: Vec::new(),
            sequence: Vec::new(),
            setting,
            simulation: 0,
        }
    }

    pub fn action(&self) -> &[Action] {
        &self.node[0].action
    }

    pub fn position(&self) -> &Position {
        &self.node[0].position
    }

    pub fn prepare(&mut self, logit: Vec<f32>, value: f64, generator: &mut Generator) {
        let root = &mut self.node[0];
        root.logit = logit;
        root.raw = value;
        root.sum = value;
        root.count = 1;
        self.gumbel = (0..root.action.len()).map(|_| generator.gumbel()).collect();
        let considered = self.setting.considered.min(root.action.len());
        self.sequence = sequence(considered, self.setting.simulation);
    }

    pub fn finished(&self) -> bool {
        self.simulation >= self.setting.simulation
    }

    pub fn leaf(&self, leaf: &Leaf) -> (&Position, &[Action]) {
        let node = &self.node[leaf.node];
        (&node.position, &node.action)
    }

    fn completed(&self, index: usize) -> Vec<f64> {
        let node = &self.node[index];
        let total = node.visit.iter().sum::<u32>();
        let prior = softmax(
            &node
                .logit
                .iter()
                .map(|&value| f64::from(value))
                .collect::<Vec<_>>(),
        );
        let quality = node
            .child
            .iter()
            .zip(&node.visit)
            .map(|(child, &visit)| {
                child
                    .filter(|_| visit > 0)
                    .map(|child| self.node[child].reward + self.node[child].value())
            })
            .collect::<Vec<_>>();
        let (weighted, mass) = quality
            .iter()
            .zip(&prior)
            .filter_map(|(quality, prior)| {
                quality.map(|quality| (quality, prior.max(f64::MIN_POSITIVE)))
            })
            .fold((0.0, 0.0), |(weighted, mass), (quality, prior)| {
                (weighted + prior * quality, mass + prior)
            });
        let mixed = if mass > 0.0 {
            (node.raw + f64::from(total) * weighted / mass) / (1.0 + f64::from(total))
        } else {
            node.raw
        };
        let completed = quality
            .iter()
            .map(|quality| quality.unwrap_or(mixed))
            .collect::<Vec<_>>();
        let minimum = completed.iter().copied().fold(f64::INFINITY, f64::min);
        let maximum = completed.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let range = (maximum - minimum).max(1e-8);
        let visit = f64::from(node.visit.iter().copied().max().unwrap_or(0));
        let scale = (self.setting.visit + visit) * self.setting.scale;
        completed
            .into_iter()
            .map(|value| scale * (value - minimum) / range)
            .collect()
    }

    fn root(&self) -> usize {
        let root = &self.node[0];
        let considered = self
            .sequence
            .get(self.simulation)
            .copied()
            .unwrap_or_else(|| root.visit.iter().copied().max().unwrap_or(0) as usize);
        self.score(|visit| visit as usize == considered)
    }

    fn score(&self, admit: impl Fn(u32) -> bool) -> usize {
        let root = &self.node[0];
        let quality = self.completed(0);
        let maximum = root.logit.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let score = |index: usize| {
            self.gumbel[index] + f64::from(root.logit[index] - maximum) + quality[index]
        };
        best(
            (0..root.action.len())
                .filter(|&index| admit(root.visit[index]))
                .map(|index| (index, score(index))),
        )
        .or_else(|| best((0..root.action.len()).map(|index| (index, score(index)))))
        .unwrap_or(0)
    }

    fn interior(&self, index: usize) -> usize {
        let node = &self.node[index];
        let quality = self.completed(index);
        let probability = softmax(
            &node
                .logit
                .iter()
                .zip(&quality)
                .map(|(&logit, quality)| f64::from(logit) + quality)
                .collect::<Vec<_>>(),
        );
        let total = f64::from(node.visit.iter().sum::<u32>());
        best(probability.iter().zip(&node.visit).enumerate().map(
            |(index, (probability, &visit))| {
                (index, probability - f64::from(visit) / (1.0 + total))
            },
        ))
        .unwrap_or(0)
    }

    fn propagate(&mut self, leaf: usize, path: &[(usize, usize)], value: f64) {
        self.node[leaf].sum += value;
        self.node[leaf].count += 1;
        let mut value = value;
        let mut child = leaf;
        for &(parent, action) in path.iter().rev() {
            value += self.node[child].reward;
            let node = &mut self.node[parent];
            node.sum += value;
            node.count += 1;
            node.visit[action] += 1;
            child = parent;
        }
        self.simulation += 1;
    }

    pub fn simulate(&mut self, environment: &mut impl Environment) -> Option<Leaf> {
        let mut path = Vec::new();
        let mut current = 0;
        loop {
            let action = if current == 0 {
                self.root()
            } else {
                self.interior(current)
            };
            path.push((current, action));
            if let Some(child) = self.node[current].child[action] {
                if self.node[child].position.evaluation.is_none() {
                    let checked = environment
                        .verify(&self.node[current].position, &self.node[child].position);
                    self.settle(child, current, checked);
                }
                if self.node[child].terminal {
                    self.propagate(child, &path, 0.0);
                    return None;
                }
                current = child;
                continue;
            }
            let parent = &self.node[current];
            let chosen = parent.action[action];
            let depth = parent.depth + 1;
            let origin = parent.position.clone();
            let terminal = chosen == Action::Stop || depth >= self.setting.step;
            let proposed = environment.transition(&origin, chosen);
            let position = if terminal && proposed.evaluation.is_none() {
                environment.verify(&origin, &proposed)
            } else {
                proposed
            };
            let reward = position.potential - origin.potential;
            let legal = if terminal {
                Vec::new()
            } else {
                environment.legal(&position)
            };
            let child = self.node.len();
            self.node
                .push(Node::new(position, reward, depth, terminal, legal));
            self.node[current].child[action] = Some(child);
            if terminal {
                self.propagate(child, &path, 0.0);
                return None;
            }
            return Some(Leaf { node: child, path });
        }
    }

    fn settle(&mut self, index: usize, parent: usize, position: Position) {
        let reward = position.potential - self.node[parent].position.potential;
        let node = &mut self.node[index];
        node.reward = reward;
        node.position = position;
    }

    pub fn parent(&self, leaf: &Leaf) -> &Position {
        let (parent, _) = leaf.path[leaf.path.len() - 1];
        &self.node[parent].position
    }

    pub fn check(&mut self, leaf: &Leaf, position: Position) {
        let (parent, _) = leaf.path[leaf.path.len() - 1];
        self.settle(leaf.node, parent, position);
    }

    pub fn expand(&mut self, leaf: Leaf, logit: Vec<f32>, value: f64, estimate: f64) {
        let (parent, _) = leaf.path[leaf.path.len() - 1];
        let origin = self.node[parent].position.potential;
        let node = &mut self.node[leaf.node];
        if node.position.evaluation.is_none() {
            node.position.potential = estimate;
            node.reward = estimate - origin;
        }
        node.logit = logit;
        node.raw = value;
        self.propagate(leaf.node, &leaf.path, value);
    }

    pub fn decide(&self) -> usize {
        let visit = self.node[0].visit.iter().copied().max().unwrap_or(0);
        self.score(|count| count == visit)
    }

    pub fn policy(&self) -> Vec<f32> {
        let root = &self.node[0];
        let quality = self.completed(0);
        softmax(
            &root
                .logit
                .iter()
                .zip(&quality)
                .map(|(&logit, quality)| f64::from(logit) + quality)
                .collect::<Vec<_>>(),
        )
        .into_iter()
        .map(|value| value as f32)
        .collect()
    }

    pub fn successor(&self, action: usize) -> Option<&Position> {
        self.node[0].child[action].map(|child| &self.node[child].position)
    }
}
