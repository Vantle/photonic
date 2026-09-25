use crate::archive::{Archive, Improvement, Record, moment};
use crate::bar::Bar;
use crate::edit::{self, Action, Bound};
use crate::encoding::{Permutation, encode};
use crate::judge::{EXPLORE, Infer, Judge, MARGIN, PROBE};
use crate::objective::{self, Evaluation, TOLERANCE, evaluate, potential};
use crate::problem::Problem;
use crate::replay::Replay;
use crate::search::{self, Environment, Leaf, Position, Tree};
use crate::server::Server;
use code::program::Program;
use network::input::{Input, Output, Sample};
use network::model::Model;
use random::Generator;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

#[derive(Clone, Copy, Debug)]
pub struct Setting {
    pub game: usize,
    pub search: search::Setting,
    pub bound: Bound,
    pub objective: objective::Setting,
    pub focus: f64,
    pub cache: usize,
    pub imitation: usize,
    pub race: usize,
    pub infer: f64,
    pub label: f64,
}

impl Default for Setting {
    fn default() -> Self {
        Self {
            game: 8,
            search: search::Setting::default(),
            bound: Bound::default(),
            objective: objective::Setting::default(),
            focus: 0.5,
            cache: 50_000,
            imitation: 4,
            race: 0,
            infer: 0.05,
            label: 0.15,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Tally {
    pub reward: f64,
    pub episode: u64,
    pub correct: u64,
}

#[derive(Default)]
pub struct Statistic {
    pub episode: AtomicU64,
    pub step: AtomicU64,
    pub evaluation: AtomicU64,
    pub inferred: AtomicU64,
    pub inference: AtomicU64,
    pub improvement: AtomicU64,
    pub tally: Mutex<Tally>,
    pub judge: Mutex<Judge>,
}

pub struct Shared {
    pub model: RwLock<Arc<Model>>,
    pub server: Option<Server>,
    pub replay: Mutex<Replay>,
    pub archive: Mutex<Archive>,
    pub problem: Vec<Problem>,
    pub focus: Vec<usize>,
    pub statistic: Statistic,
    pub stop: AtomicBool,
    pub discovery: Mutex<Vec<Improvement>>,
}

struct Cache {
    entry: HashMap<(usize, Arc<Program>), Arc<Evaluation>>,
    capacity: usize,
}

impl Cache {
    fn known(&self, index: usize, program: &Arc<Program>) -> bool {
        self.entry.contains_key(&(index, program.clone()))
    }

    fn assess(
        &mut self,
        problem: &[Problem],
        index: usize,
        program: &Arc<Program>,
        setting: &objective::Setting,
        statistic: &Statistic,
    ) -> Arc<Evaluation> {
        let key = (index, program.clone());
        if let Some(known) = self.entry.get(&key) {
            return known.clone();
        }
        if self.entry.len() >= self.capacity {
            self.entry.clear();
        }
        statistic.evaluation.fetch_add(1, Ordering::Relaxed);
        let task = &problem[index].task;
        let result = Arc::new(evaluate(
            program,
            &task.example,
            &task.vocabulary,
            &setting.aim(task.goal),
        ));
        self.entry.insert(key, result.clone());
        result
    }
}

struct Found {
    problem: usize,
    program: Arc<Program>,
    evaluation: Arc<Evaluation>,
}

struct Label {
    problem: usize,
    program: Arc<Program>,
    parent: Arc<Evaluation>,
    potential: f64,
}

struct Simulator<'worker> {
    problem: &'worker [Problem],
    index: usize,
    cache: &'worker mut Cache,
    setting: &'worker Setting,
    statistic: &'worker Statistic,
    bar: &'worker mut Bar,
    found: &'worker mut Vec<Found>,
    label: &'worker mut Vec<Label>,
    generator: &'worker mut Generator,
    infer: Infer,
}

impl Simulator<'_> {
    fn exact(&mut self, parent: &Position, program: Arc<Program>) -> Position {
        let fresh = !self.cache.known(self.index, &program);
        let evaluation = self.cache.assess(
            self.problem,
            self.index,
            &program,
            &self.setting.objective,
            self.statistic,
        );
        if self.bar.raise(self.index, &program, &evaluation) {
            self.found.push(Found {
                problem: self.index,
                program: program.clone(),
                evaluation: evaluation.clone(),
            });
        }
        let value = potential(&evaluation, self.problem[self.index].baseline);
        if let Some(known) = parent.evaluation.as_ref()
            && fresh
            && self.generator.chance(self.setting.label)
        {
            self.label.push(Label {
                problem: self.index,
                program: program.clone(),
                parent: known.clone(),
                potential: value,
            });
        }
        Position {
            potential: value,
            program,
            evaluation: Some(evaluation),
        }
    }
}

impl Environment for Simulator<'_> {
    fn transition(&mut self, position: &Position, action: Action) -> Position {
        if action == Action::Stop {
            return position.clone();
        }
        let program = Arc::new(edit::apply(&position.program, action));
        let blind = match self.infer {
            Infer::Never => false,
            Infer::Probe => self.generator.chance(PROBE),
            Infer::Trusted => true,
        };
        if blind && !self.cache.known(self.index, &program) {
            self.statistic.inferred.fetch_add(1, Ordering::Relaxed);
            return Position {
                program,
                evaluation: None,
                potential: f64::NAN,
            };
        }
        self.exact(position, program)
    }

    fn verify(&mut self, parent: &Position, position: &Position) -> Position {
        if position.evaluation.is_some() {
            return position.clone();
        }
        let checked = self.exact(parent, position.program.clone());
        if position.potential.is_finite() {
            self.statistic
                .judge
                .lock()
                .expect("the judge lock is never poisoned")
                .record((position.potential - checked.potential).abs());
        }
        checked
    }

    fn legal(&mut self, position: &Position) -> Vec<Action> {
        let problem = &self.problem[self.index];
        edit::legal(
            &position.program,
            problem.task.vocabulary.len(),
            &bound(&self.setting.bound, problem),
        )
    }
}

pub fn bound(bound: &Bound, problem: &Problem) -> Bound {
    Bound {
        depth: bound.depth.max(problem.nesting),
        ..*bound
    }
}

struct Pending {
    input: Input,
    policy: Vec<f32>,
    potential: f64,
    chosen: usize,
    improving: bool,
}

struct Game {
    problem: usize,
    permutation: Permutation,
    position: Position,
    depth: usize,
    start: f64,
    record: Vec<Pending>,
    network: HashMap<(Arc<Program>, bool), Output>,
    tree: Option<Tree>,
    root: Option<Input>,
    idle: bool,
}

fn outcome(left: &Game, right: &Game) -> std::cmp::Ordering {
    let difference = left.position.potential - right.position.potential;
    if difference.abs() > TOLERANCE {
        return difference.total_cmp(&0.0);
    }
    right.depth.cmp(&left.depth)
}

pub struct Worker {
    shared: Arc<Shared>,
    setting: Setting,
    generator: Generator,
    cache: Cache,
    bar: Bar,
    found: Vec<Found>,
    label: Vec<Label>,
    sample: Vec<Sample>,
    game: Vec<Game>,
}

impl Worker {
    pub fn new(shared: Arc<Shared>, setting: Setting, seed: u64) -> Self {
        let count = shared.problem.len();
        let mut worker = Self {
            shared,
            setting,
            generator: Generator::new(seed),
            cache: Cache {
                entry: HashMap::new(),
                capacity: setting.cache,
            },
            bar: Bar::new(count, setting.cache),
            found: Vec::new(),
            label: Vec::new(),
            sample: Vec::new(),
            game: Vec::new(),
        };
        worker.refresh();
        worker.game = if setting.race > 0 {
            (0..setting.game.div_ceil(2))
                .flat_map(|_| worker.couple())
                .collect()
        } else {
            (0..setting.game).map(|_| worker.episode()).collect()
        };
        worker
    }

    fn refresh(&mut self) {
        let archive = self
            .shared
            .archive
            .lock()
            .expect("the archive lock is never poisoned");
        self.bar.refresh(&archive, &self.shared.problem);
    }

    fn choose(&mut self) -> usize {
        if !self.shared.focus.is_empty() && self.generator.chance(self.setting.focus) {
            return self.shared.focus[self.generator.below(self.shared.focus.len())];
        }
        self.generator.below(self.shared.problem.len())
    }

    fn start(&mut self, index: usize) -> Arc<Program> {
        let problem = &self.shared.problem[index];
        let (best, partial) = {
            let archive = self
                .shared
                .archive
                .lock()
                .expect("the archive lock is never poisoned");
            archive
                .entry(&problem.task.name)
                .map_or((None, None), |entry| {
                    (
                        entry.best.as_ref().map(|record| record.program.clone()),
                        entry.partial.as_ref().map(|record| record.program.clone()),
                    )
                })
        };
        let reference = problem.task.reference.clone().unwrap_or_default();
        let draw = self.generator.uniform();
        let program = if draw < 0.15 {
            Program::default()
        } else if draw < 0.4 {
            reference
        } else if draw < 0.7 {
            best.unwrap_or(reference)
        } else if draw < 0.85 {
            let count = 1 + self.generator.below(3);
            edit::perturb(
                &best.unwrap_or(reference),
                problem.task.vocabulary.len(),
                &bound(&self.setting.bound, problem),
                count,
                &mut self.generator,
            )
        } else {
            partial.unwrap_or_default()
        };
        Arc::new(program)
    }

    fn episode(&mut self) -> Game {
        let index = self.choose();
        let program = self.start(index);
        let evaluation = self.cache.assess(
            &self.shared.problem,
            index,
            &program,
            &self.setting.objective,
            &self.shared.statistic,
        );
        let problem = &self.shared.problem[index];
        let value = potential(&evaluation, problem.baseline);
        Game {
            problem: index,
            permutation: Permutation::new(&mut self.generator, problem.task.example.len()),
            position: Position {
                program,
                evaluation: Some(evaluation),
                potential: value,
            },
            depth: 0,
            start: value,
            record: Vec::new(),
            network: HashMap::new(),
            tree: None,
            root: None,
            idle: false,
        }
    }

    fn couple(&mut self) -> [Game; 2] {
        let first = self.episode();
        let example = self.shared.problem[first.problem].task.example.len();
        let second = Game {
            problem: first.problem,
            permutation: Permutation::new(&mut self.generator, example),
            position: first.position.clone(),
            depth: 0,
            start: first.start,
            record: Vec::new(),
            network: HashMap::new(),
            tree: None,
            root: None,
            idle: false,
        };
        [first, second]
    }

    fn infer(&self, model: &Model, input: Vec<Input>) -> Vec<Output> {
        if input.is_empty() {
            return Vec::new();
        }
        self.shared
            .statistic
            .inference
            .fetch_add(input.len() as u64, Ordering::Relaxed);
        let input = Arc::new(input);
        self.shared
            .server
            .as_ref()
            .and_then(|server| server.infer(input.clone()))
            .unwrap_or_else(|| model.infer(&input.iter().collect::<Vec<_>>()))
    }

    fn inference(&self) -> Infer {
        self.shared
            .statistic
            .judge
            .lock()
            .expect("the judge lock is never poisoned")
            .mode(self.setting.infer)
    }

    fn begin(&mut self, model: &Model) {
        let mut request = Vec::new();
        let mut input = Vec::new();
        for (index, game) in self.game.iter_mut().enumerate() {
            if game.tree.is_some() || game.idle {
                continue;
            }
            let problem = &self.shared.problem[game.problem];
            let action = edit::legal(
                &game.position.program,
                problem.task.vocabulary.len(),
                &bound(&self.setting.bound, problem),
            );
            let encoded = encode(
                &problem.task,
                &game.position.program,
                game.position
                    .evaluation
                    .as_ref()
                    .expect("every game advances to a verified position"),
                &action,
                &game.permutation,
            );
            game.tree = Some(Tree::new(
                game.position.clone(),
                game.depth,
                action,
                self.setting.search,
            ));
            game.root = Some(encoded.clone());
            match game.network.get(&(game.position.program.clone(), true)) {
                Some(output) => {
                    let (logit, value) = (output.logit.clone(), f64::from(output.value));
                    if let Some(tree) = game.tree.as_mut() {
                        tree.prepare(logit, value, &mut self.generator);
                    }
                }
                None => {
                    request.push(index);
                    input.push(encoded);
                }
            }
        }
        let output = self.infer(model, input);
        for (index, output) in request.into_iter().zip(output) {
            let game = &mut self.game[index];
            let (logit, value) = (output.logit.clone(), f64::from(output.value));
            game.network
                .insert((game.position.program.clone(), true), output);
            if let Some(tree) = game.tree.as_mut() {
                tree.prepare(logit, value, &mut self.generator);
            }
        }
    }

    fn simulator(&mut self, index: usize, infer: Infer) -> Simulator<'_> {
        Simulator {
            problem: &self.shared.problem,
            index,
            cache: &mut self.cache,
            setting: &self.setting,
            statistic: &self.shared.statistic,
            bar: &mut self.bar,
            found: &mut self.found,
            label: &mut self.label,
            generator: &mut self.generator,
            infer,
        }
    }

    fn select(&mut self, infer: Infer) -> Vec<(usize, Leaf)> {
        let mut result = Vec::new();
        for index in 0..self.game.len() {
            let Some(mut tree) = self.game[index].tree.take() else {
                continue;
            };
            let problem = self.game[index].problem;
            if !tree.finished()
                && let Some(leaf) = tree.simulate(&mut self.simulator(problem, infer))
            {
                result.push((index, leaf));
            }
            self.game[index].tree = Some(tree);
        }
        result
    }

    fn position(&self, index: usize, leaf: &Leaf) -> &Position {
        self.game[index]
            .tree
            .as_ref()
            .expect("a selected leaf belongs to a live search")
            .leaf(leaf)
            .0
    }

    fn consult(&mut self, model: &Model, leaf: &[(usize, Leaf)]) -> Vec<Output> {
        let mut known = Vec::with_capacity(leaf.len());
        let mut input = Vec::new();
        for (index, leaf) in leaf {
            let game = &self.game[*index];
            let (position, action) = game
                .tree
                .as_ref()
                .expect("a selected leaf belongs to a live search")
                .leaf(leaf);
            let key = (position.program.clone(), position.evaluation.is_some());
            let cached = game.network.get(&key).cloned();
            if cached.is_none() {
                let task = &self.shared.problem[game.problem].task;
                let evaluation = match &position.evaluation {
                    Some(evaluation) => evaluation,
                    None => game
                        .tree
                        .as_ref()
                        .expect("a selected leaf belongs to a live search")
                        .parent(leaf)
                        .evaluation
                        .as_ref()
                        .expect("every expanded parent is verified"),
                };
                input.push(encode(
                    task,
                    &position.program,
                    evaluation,
                    action,
                    &game.permutation,
                ));
            }
            known.push(cached);
        }
        let mut fresh = self.infer(model, input).into_iter();
        let mut result = Vec::with_capacity(leaf.len());
        for ((index, leaf), cached) in leaf.iter().zip(known) {
            if let Some(output) = cached {
                result.push(output);
                continue;
            }
            let output = fresh
                .next()
                .expect("the network answers every uncached leaf");
            let position = self.position(*index, leaf);
            let key = (position.program.clone(), position.evaluation.is_some());
            self.game[*index].network.insert(key, output.clone());
            result.push(output);
        }
        result
    }

    fn parent(&self, index: usize, leaf: &Leaf) -> f64 {
        self.game[index]
            .tree
            .as_ref()
            .expect("a selected leaf belongs to a live search")
            .parent(leaf)
            .potential
    }

    fn doubtful(&self, index: usize, leaf: &Leaf, output: &Output) -> bool {
        let [_, upper] = output.judge[..] else {
            return false;
        };
        f64::from(upper) < self.parent(index, leaf) - MARGIN
    }

    fn confirm(&mut self, index: usize, leaf: &Leaf, output: &Output, doubtful: bool, weight: f64) {
        let parent = self.parent(index, leaf);
        let Some(mut tree) = self.game[index].tree.take() else {
            return;
        };
        let proposed = Position {
            potential: output
                .judge
                .first()
                .map_or(f64::NAN, |&median| f64::from(median)),
            ..tree.leaf(leaf).0.clone()
        };
        let problem = self.game[index].problem;
        let checked = self
            .simulator(problem, Infer::Never)
            .verify(tree.parent(leaf), &proposed);
        if checked.potential >= parent - MARGIN {
            self.shared
                .statistic
                .judge
                .lock()
                .expect("the judge lock is never poisoned")
                .observe(doubtful, weight);
        }
        tree.check(leaf, checked);
        self.game[index].tree = Some(tree);
    }

    fn expand(&mut self, index: usize, leaf: Leaf, output: Output) {
        let estimate = output
            .judge
            .first()
            .map_or(f64::NAN, |&median| f64::from(median));
        if let Some(tree) = self.game[index].tree.as_mut() {
            tree.expand(leaf, output.logit, f64::from(output.value), estimate);
        }
    }

    fn simulate(&mut self, model: &Model) {
        let infer = self.inference();
        let leaf = self.select(infer);
        let (open, mut known): (Vec<_>, Vec<_>) = leaf
            .into_iter()
            .partition(|(index, leaf)| self.position(*index, leaf).evaluation.is_none());
        let judged = self.consult(model, &open);
        let mut blind = Vec::new();
        for ((index, leaf), output) in open.into_iter().zip(judged) {
            let doubtful = self.doubtful(index, &leaf, &output);
            let audit = infer == Infer::Trusted && doubtful;
            if audit && !self.generator.chance(EXPLORE) {
                self.shared
                    .statistic
                    .judge
                    .lock()
                    .expect("the judge lock is never poisoned")
                    .skipped += 1;
                blind.push((index, leaf, output));
                continue;
            }
            let weight = if audit { 1.0 / EXPLORE } else { 1.0 };
            self.confirm(index, &leaf, &output, doubtful, weight);
            known.push((index, leaf));
        }
        let informed = self.consult(model, &known);
        for ((index, leaf), output) in known.into_iter().zip(informed) {
            self.expand(index, leaf, output);
        }
        for (index, leaf, output) in blind {
            self.expand(index, leaf, output);
        }
    }

    fn decide(&mut self) {
        for index in 0..self.game.len() {
            let Some(tree) = self.game[index].tree.take() else {
                continue;
            };
            let chosen = tree.decide();
            let action = tree.action()[chosen];
            let policy = tree.policy();
            let next = {
                let problem = self.game[index].problem;
                let mut simulator = self.simulator(problem, Infer::Never);
                match tree.successor(chosen) {
                    Some(position) => simulator.verify(tree.position(), position),
                    None => simulator.transition(tree.position(), action),
                }
            };
            let entry = &mut self.game[index];
            let input = entry
                .root
                .take()
                .expect("every searched position records its encoding");
            entry.record.push(Pending {
                input,
                policy,
                potential: entry.position.potential,
                chosen,
                improving: next
                    .evaluation
                    .as_ref()
                    .is_some_and(|evaluation| evaluation.correct)
                    && next.potential > entry.position.potential + TOLERANCE,
            });
            entry.depth += 1;
            entry.position = next;
            self.shared.statistic.step.fetch_add(1, Ordering::Relaxed);
            if action == Action::Stop || entry.depth >= self.setting.search.step {
                self.finish(index);
            }
        }
    }

    fn finish(&mut self, index: usize) {
        if self.setting.race == 0 {
            let fresh = self.episode();
            let game = std::mem::replace(&mut self.game[index], fresh);
            self.conclude(game, false);
            return;
        }
        self.game[index].idle = true;
        let rival = index ^ 1;
        if self.game.get(rival).is_some_and(|game| !game.idle) {
            return;
        }
        let low = index & !1;
        let [first, second] = self.couple();
        let left = std::mem::replace(&mut self.game[low], first);
        let right = std::mem::replace(&mut self.game[low + 1], second);
        let order = outcome(&left, &right);
        self.conclude(left, order.is_gt());
        self.conclude(right, order.is_lt());
    }

    fn conclude(&mut self, game: Game, won: bool) {
        let last = game.position.potential;
        for pending in game.record {
            let value = (last - pending.potential) as f32;
            if won {
                let mut policy = vec![0.0; pending.policy.len()];
                policy[pending.chosen] = 1.0;
                for _ in 0..self.setting.race {
                    self.sample.push(Sample {
                        input: pending.input.clone(),
                        policy: policy.clone(),
                        value: Some(value),
                        judge: None,
                    });
                }
            }
            if pending.improving {
                let mut policy = vec![0.0; pending.policy.len()];
                policy[pending.chosen] = 1.0;
                for _ in 0..self.setting.imitation {
                    self.sample.push(Sample {
                        input: pending.input.clone(),
                        policy: policy.clone(),
                        value: Some(value),
                        judge: None,
                    });
                }
            }
            self.sample.push(Sample {
                input: pending.input,
                policy: pending.policy,
                value: Some(value),
                judge: None,
            });
        }
        let statistic = &self.shared.statistic;
        statistic.episode.fetch_add(1, Ordering::Relaxed);
        let mut tally = statistic
            .tally
            .lock()
            .expect("the tally lock is never poisoned");
        tally.reward += last - game.start;
        tally.episode += 1;
        tally.correct += u64::from(
            game.position
                .evaluation
                .as_ref()
                .is_some_and(|evaluation| evaluation.correct),
        );
    }

    fn flush(&mut self) {
        for label in std::mem::take(&mut self.label) {
            let task = &self.shared.problem[label.problem].task;
            let permutation = Permutation::new(&mut self.generator, task.example.len());
            self.sample.push(Sample {
                input: encode(task, &label.program, &label.parent, &[], &permutation),
                policy: Vec::new(),
                value: None,
                judge: Some(label.potential as f32),
            });
        }
        if !self.sample.is_empty() {
            let mut replay = self
                .shared
                .replay
                .lock()
                .expect("the replay lock is never poisoned");
            for sample in self.sample.drain(..) {
                replay.push(sample);
            }
        }
        if self.found.is_empty() {
            return;
        }
        let found = std::mem::take(&mut self.found);
        let mut improvement = Vec::new();
        for entry in found {
            let task = &self.shared.problem[entry.problem].task;
            let general = entry.evaluation.correct
                && objective::general(&entry.program, task, &self.setting.objective);
            if entry.evaluation.correct && !general {
                self.bar.reject(entry.problem, entry.program);
                continue;
            }
            let record = Record::new(
                entry.program.as_ref().clone(),
                &entry.evaluation,
                general,
                moment(),
            );
            let offered = self
                .shared
                .archive
                .lock()
                .expect("the archive lock is never poisoned")
                .offer(&task.name, record);
            improvement.extend(offered);
        }
        if improvement.is_empty() {
            return;
        }
        self.shared
            .statistic
            .improvement
            .fetch_add(improvement.len() as u64, Ordering::Relaxed);
        self.shared
            .discovery
            .lock()
            .expect("the discovery lock is never poisoned")
            .extend(improvement);
    }

    pub fn run(&mut self) {
        while !self.shared.stop.load(Ordering::Relaxed) {
            let model = self
                .shared
                .model
                .read()
                .expect("the model lock is never poisoned")
                .clone();
            self.refresh();
            self.begin(&model);
            for _ in 0..self.setting.search.simulation {
                if self.shared.stop.load(Ordering::Relaxed) {
                    return;
                }
                self.simulate(&model);
            }
            self.decide();
            self.flush();
        }
    }
}
