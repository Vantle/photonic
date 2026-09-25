use crate::objective::{Evaluation, Setting, TOLERANCE, evaluate, floor};
use crate::task::{Example, Task};
use code::atom::Atom;
use code::output::Output;
use code::particle::Particle;
use code::program::Program;
use code::rule::Rule;
use machine::event::{apply, enumerate};
use machine::exploration::fits;
use machine::flat::Flat;
use machine::state::State;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use thiserror::Error;

const ALPHABET: usize = 24;

#[derive(Debug, Error, PartialEq)]
pub enum Failure {
    #[error("without a positive size weight no program size bound exists")]
    Unbounded,
    #[error(
        "the vocabulary names {count} atoms, more than the {ALPHABET} whose subsets are enumerated"
    )]
    Alphabet { count: usize },
}

#[derive(Clone, Copy, Debug)]
pub struct Budget {
    pub size: usize,
    pub time: Option<Duration>,
}

impl Default for Budget {
    fn default() -> Self {
        Self {
            size: 16,
            time: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Solution {
    pub floor: f64,
    pub weight: f64,
    pub size: Option<usize>,
    pub examined: u64,
    pub cost: f64,
    pub optimal: Vec<(Program, Evaluation)>,
    pub proven: bool,
    pub complete: bool,
}

fn reach(floor: f64, weight: f64, size: Option<usize>) -> f64 {
    floor + weight * size.map_or(0, |size| size + 1) as f64
}

pub fn least(task: &Task, setting: &Setting) -> f64 {
    let setting = setting.aim(task.goal);
    if setting.goal.size < 0.0 {
        return f64::NEG_INFINITY;
    }
    floor(&task.example, &task.vocabulary, &setting)
}

impl Solution {
    pub fn gap(&self) -> f64 {
        reach(self.floor, self.weight, self.size)
    }
}

struct Candidate {
    rule: Rule,
    cost: usize,
    mask: u64,
    need: u64,
    make: u64,
}

struct Tally {
    examined: u64,
    cost: f64,
    optimal: Vec<(Program, Evaluation)>,
}

impl Tally {
    fn empty() -> Self {
        Self {
            examined: 0,
            cost: f64::INFINITY,
            optimal: Vec::new(),
        }
    }

    fn offer(&mut self, program: Program, evaluation: Evaluation) {
        if evaluation.cost < self.cost - TOLERANCE {
            self.cost = evaluation.cost;
            self.optimal.clear();
        }
        if evaluation.cost <= self.cost + TOLERANCE {
            self.optimal.push((program, evaluation));
        }
    }

    fn merge(mut self, other: Self) -> Self {
        self.examined += other.examined;
        for (program, evaluation) in other.optimal {
            self.offer(program, evaluation);
        }
        self
    }
}

struct Scope<'scope> {
    task: &'scope Task,
    setting: &'scope Setting,
    present: u64,
    deadline: Option<Instant>,
    expired: AtomicBool,
}

struct Branch<'branch> {
    scope: &'branch Scope<'branch>,
    candidate: &'branch [Candidate],
    full: u64,
    budget: usize,
    bound: f64,
}

fn walk(flat: &Flat, example: &Example, setting: &Setting) -> bool {
    let limit = &setting.limit;
    let Some(mut state) = State::new(&example.input) else {
        return true;
    };
    let mut work = 0;
    loop {
        let mut first = None;
        let mut count = 0;
        let complete = enumerate(flat, &state, |event| {
            count += 1;
            first.get_or_insert(event);
            count <= limit.event
        });
        if !complete {
            return false;
        }
        let Some(event) = first else {
            let budget = limit.individualization;
            return state.observation().same(&example.output, budget);
        };
        if work >= limit.state {
            return false;
        }
        work += 1;
        state = apply(flat, &state, &[event]);
        if !fits(flat, &state, limit) {
            return false;
        }
    }
}

fn choose(count: usize, size: usize) -> Vec<u64> {
    (0..1u64 << count)
        .filter(|mask| mask.count_ones() as usize == size)
        .collect()
}

fn form(atom: &[Atom], limit: usize, start: usize, current: &mut Vec<Atom>) -> Vec<Vec<Atom>> {
    let mut result = vec![current.clone()];
    if current.len() == limit {
        return result;
    }
    for index in start..atom.len() {
        current.push(atom[index]);
        result.extend(form(atom, limit, index, current));
        current.pop();
    }
    result
}

fn gather(
    particle: &[Vec<Atom>],
    budget: usize,
    start: usize,
    current: &mut Vec<usize>,
    cost: usize,
) -> Vec<(Vec<usize>, usize)> {
    let mut result = vec![(current.clone(), cost)];
    for index in start..particle.len() {
        let next = cost + 1 + particle[index].len();
        if next > budget {
            break;
        }
        current.push(index);
        result.extend(gather(particle, budget, index, current, next));
        current.pop();
    }
    result
}

fn mask<'particle>(particle: impl Iterator<Item = &'particle Particle>) -> u64 {
    particle
        .flat_map(|particle| particle.flat().unwrap_or_default())
        .fold(0, |mask, atom| mask | 1 << atom.index())
}

fn live(candidate: &[Candidate], chosen: &[usize], present: u64) -> bool {
    let mut available = present;
    let mut fired = vec![false; chosen.len()];
    loop {
        let mut changed = false;
        for (index, &rule) in chosen.iter().enumerate() {
            if !fired[index] && candidate[rule].need & !available == 0 {
                fired[index] = true;
                available |= candidate[rule].make;
                changed = true;
            }
        }
        if !changed {
            return fired.iter().all(|&fired| fired);
        }
    }
}

fn assemble(atom: &[Atom], budget: usize) -> Vec<Candidate> {
    let mut particle = form(atom, budget.saturating_sub(2), 0, &mut Vec::new());
    particle.sort_by_key(Vec::len);
    let bundle = gather(&particle, budget.saturating_sub(1), 0, &mut Vec::new(), 0);
    let build = |choice: &[usize]| {
        choice
            .iter()
            .map(|&index| Particle::atom(&particle[index]))
            .collect::<Vec<_>>()
    };
    let mut result = Vec::new();
    for (input, left) in bundle.iter().filter(|(choice, _)| !choice.is_empty()) {
        for (output, right) in &bundle {
            let cost = 1 + left + right;
            if cost > budget {
                continue;
            }
            let rule = Rule::new(
                build(input),
                build(output).into_iter().map(Output::plain).collect(),
            );
            let need = mask(rule.input().iter());
            let make = mask(rule.output().iter().map(Output::particle));
            result.push(Candidate {
                mask: need | make,
                need,
                make,
                rule,
                cost,
            });
        }
    }
    result.sort_by(|left, right| {
        left.cost
            .cmp(&right.cost)
            .then_with(|| left.rule.cmp(&right.rule))
    });
    result
}

impl Scope<'_> {
    fn expired(&self) -> bool {
        if self.expired.load(Ordering::Relaxed) {
            return true;
        }
        let passed = self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline);
        if passed {
            self.expired.store(true, Ordering::Relaxed);
        }
        passed
    }

    fn assess(&self, program: &Program) -> Option<Evaluation> {
        if let Some(flat) = Flat::new(program)
            && !self
                .task
                .example
                .iter()
                .all(|example| walk(&flat, example, self.setting))
        {
            return None;
        }
        for example in &self.task.example {
            let single = evaluate(
                program,
                std::slice::from_ref(example),
                &self.task.vocabulary,
                self.setting,
            );
            if !single.correct {
                return None;
            }
        }
        let evaluation = evaluate(
            program,
            &self.task.example,
            &self.task.vocabulary,
            self.setting,
        );
        evaluation.correct.then_some(evaluation)
    }

    fn visit(&self, program: Program, bound: f64, tally: &mut Tally) {
        let Some(evaluation) = self.assess(&program) else {
            return;
        };
        if evaluation.cost <= bound + TOLERANCE {
            tally.offer(program, evaluation);
        }
    }

    fn explore(&self, full: u64, size: usize, bound: f64) -> Tally {
        let budget = size - full.count_ones() as usize;
        let atom = (0..ALPHABET)
            .filter(|&bit| full & 1 << bit != 0)
            .map(|bit| Atom(bit as u16))
            .collect::<Vec<_>>();
        let candidate = assemble(&atom, budget);
        let branch = Branch {
            scope: self,
            candidate: &candidate,
            full,
            budget,
            bound,
        };
        (0..candidate.len())
            .into_par_iter()
            .map(|first| {
                let mut tally = Tally::empty();
                branch.descend(
                    &mut vec![first],
                    candidate[first].cost,
                    candidate[first].mask,
                    &mut tally,
                );
                tally
            })
            .reduce(Tally::empty, Tally::merge)
    }

    fn level(&self, size: usize, bound: f64) -> Tally {
        if size == 0 {
            let mut tally = Tally::empty();
            tally.examined += 1;
            self.visit(Program::default(), bound, &mut tally);
            return tally;
        }
        (0..=size.min(self.task.vocabulary.len()))
            .flat_map(|used| choose(self.task.vocabulary.len(), used))
            .collect::<Vec<_>>()
            .into_par_iter()
            .map(|full| self.explore(full, size, bound))
            .reduce(Tally::empty, Tally::merge)
    }
}

impl Branch<'_> {
    fn descend(&self, chosen: &mut Vec<usize>, cost: usize, mask: u64, tally: &mut Tally) {
        if self.scope.expired() {
            return;
        }
        if cost == self.budget && mask == self.full {
            tally.examined += 1;
            if live(self.candidate, chosen, self.scope.present) {
                let program = Program::from(
                    chosen
                        .iter()
                        .map(|&index| self.candidate[index].rule.clone())
                        .collect::<Vec<_>>(),
                );
                self.scope.visit(program, self.bound, tally);
            }
        }
        let start = chosen.last().copied().unwrap_or(0);
        for index in start..self.candidate.len() {
            let next = cost + self.candidate[index].cost;
            if next > self.budget {
                break;
            }
            chosen.push(index);
            self.descend(chosen, next, mask | self.candidate[index].mask, tally);
            chosen.pop();
        }
    }
}

pub fn solve(
    task: &Task,
    incumbent: f64,
    setting: &Setting,
    budget: Budget,
) -> Result<Solution, Failure> {
    let setting = &setting.aim(task.goal);
    if setting.goal.size <= 0.0 {
        return Err(Failure::Unbounded);
    }
    let count = task.vocabulary.len();
    if count > ALPHABET {
        return Err(Failure::Alphabet { count });
    }
    let floor = floor(&task.example, &task.vocabulary, setting);
    let scope = Scope {
        task,
        setting,
        present: mask(
            task.example
                .iter()
                .flat_map(|example| example.input.coherence()),
        ),
        deadline: budget.time.map(|time| Instant::now() + time),
        expired: AtomicBool::new(false),
    };
    let mut tally = Tally::empty();
    let mut size = None;
    for level in 0..=budget.size {
        let bound = incumbent.min(tally.cost);
        if floor + setting.goal.size * level as f64 > bound + TOLERANCE || scope.expired() {
            break;
        }
        tally = tally.merge(scope.level(level, bound));
        if scope.expired() {
            break;
        }
        size = Some(level);
    }
    let bound = incumbent.min(tally.cost);
    let least = reach(floor, setting.goal.size, size);
    let mut optimal = tally.optimal;
    optimal.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(Solution {
        floor,
        weight: setting.goal.size,
        size,
        examined: tally.examined,
        cost: tally.cost,
        optimal,
        proven: size.is_some() && least > bound - TOLERANCE,
        complete: size.is_some() && least > bound + TOLERANCE,
    })
}
