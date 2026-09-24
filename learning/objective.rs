use crate::task::{Example, Goal};
use code::canonical::Key;
use code::configuration::Configuration;
use code::distance::distance;
use code::hashing::value;
use code::observation::Observation;
use code::output::Output;
use code::program::Program;
use code::rule::Rule;
use code::tree::walk;
use code::value::Value;
use machine::exploration::explore;
use machine::flat::Flat;
use machine::limit::Limit;
use machine::schedule::schedule;
use machine::state::State;
use random::Generator;
use serde::Serialize;
use translation::execution;
use translation::vocabulary::Vocabulary;

const FLOOR: f64 = 0.05;

#[derive(Clone, Copy, Debug)]
pub struct Setting {
    pub processor: f64,
    pub size: f64,
    pub limit: Limit,
    pub admission: photonic::runtime::Limit,
    pub bound: photonic::execution::Bound,
    pub sample: usize,
    pub budget: usize,
}

impl Default for Setting {
    fn default() -> Self {
        Self {
            processor: 4.0,
            size: 0.05,
            limit: Limit {
                state: 512,
                coherence: 32,
                cell: 256,
                event: 1_024,
                ..Limit::default()
            },
            admission: photonic::runtime::Limit {
                state: 512,
                record: 10_000_000,
                world: 32,
                cell: 256,
                frame: 32,
            },
            bound: photonic::execution::Bound {
                state: 256,
                ..photonic::execution::Bound::default()
            },
            sample: 2,
            budget: 8_192,
        }
    }
}

impl Setting {
    pub fn aim(&self, goal: Option<Goal>) -> Self {
        goal.map_or(*self, |goal| Self {
            processor: goal.processor,
            size: goal.size,
            ..*self
        })
    }

    pub fn thorough(&self) -> Self {
        Self {
            limit: Limit {
                state: 16_384,
                coherence: 64,
                cell: 1_024,
                event: 65_536,
                ..self.limit
            },
            admission: photonic::runtime::Limit {
                world: 64,
                cell: 1_024,
                frame: 64,
                ..self.admission
            },
            bound: photonic::execution::Bound {
                state: 16_384,
                ..self.bound
            },
            sample: 16,
            budget: usize::MAX,
            ..*self
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum Outcome {
    Exact {
        work: usize,
        span: usize,
        verified: bool,
    },
    Different {
        output: Configuration,
        distance: f64,
    },
    Choice {
        output: Configuration,
        distance: f64,
    },
    Divergent,
}

impl Outcome {
    pub fn distance(&self) -> f64 {
        match self {
            Self::Exact { .. } => 0.0,
            Self::Different { distance, .. } | Self::Choice { distance, .. } => *distance,
            Self::Divergent => 1.0,
        }
    }

    fn time(&self, processor: f64) -> f64 {
        match self {
            Self::Exact { work, span, .. } => *span as f64 + *work as f64 / processor,
            _ => 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub struct Size {
    pub atom: usize,
    pub rule: usize,
    pub particle: usize,
    pub occurrence: usize,
}

impl Size {
    pub fn new(program: &Program) -> Self {
        let node = walk(program);
        let particle = |rule: &code::rule::Rule| {
            rule.input()
                .iter()
                .chain(rule.output().iter().map(code::output::Output::particle))
                .cloned()
                .collect::<Vec<_>>()
        };
        let mut atom = node
            .iter()
            .flat_map(|node| particle(node.rule))
            .flat_map(|particle| {
                particle
                    .value()
                    .iter()
                    .filter_map(Value::atom)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        atom.sort_unstable();
        atom.dedup();
        Self {
            atom: atom.len(),
            rule: node.len(),
            particle: node
                .iter()
                .map(|node| node.rule.input().len() + node.rule.output().len())
                .sum(),
            occurrence: node
                .iter()
                .flat_map(|node| particle(node.rule))
                .map(|particle| particle.len())
                .sum(),
        }
    }

    pub fn total(&self) -> usize {
        self.atom + self.rule + self.particle + self.occurrence
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Evaluation {
    pub outcome: Vec<Outcome>,
    pub correctness: f64,
    pub correct: bool,
    pub verified: bool,
    pub time: f64,
    pub work: f64,
    pub span: f64,
    pub size: Size,
    pub cost: f64,
}

struct Trial {
    terminal: Vec<Observation>,
    state: usize,
    cycle: bool,
    overflow: bool,
    truncated: bool,
}

struct Run {
    terminal: Option<Observation>,
    work: usize,
    depth: usize,
}

struct Context<'context> {
    program: &'context Program,
    flat: Option<Flat>,
    vocabulary: &'context Vocabulary,
    setting: &'context Setting,
}

impl Context<'_> {
    fn trial(
        &self,
        input: &Configuration,
        remaining: usize,
        expected: Option<&Key<Value>>,
    ) -> Trial {
        let budget = self.setting.limit.individualization;
        let wrong =
            |observation: &Observation| expected.is_some_and(|key| observation.key(budget) != *key);
        if let (Some(flat), Some(initial)) = (&self.flat, State::new(input)) {
            let limit = Limit {
                state: self.setting.limit.state.min(remaining),
                ..self.setting.limit
            };
            let exploration = explore(flat, initial, &limit, |state| wrong(&state.observation()));
            return Trial {
                terminal: exploration
                    .terminal
                    .iter()
                    .map(State::observation)
                    .collect(),
                state: exploration.state,
                cycle: exploration.cycle,
                overflow: exploration.overflow,
                truncated: exploration.truncated,
            };
        }
        let exploration = execution::explore(
            self.program,
            input,
            self.vocabulary,
            self.setting.admission,
            photonic::execution::Bound {
                state: self.setting.bound.state.min(remaining),
                ..self.setting.bound
            },
            wrong,
        );
        Trial {
            terminal: exploration.terminal,
            state: exploration.state,
            cycle: exploration.cycle,
            overflow: exploration.overflow,
            truncated: exploration.truncated,
        }
    }

    fn run(&self, input: &Configuration, choose: impl FnMut(usize) -> usize) -> Option<Run> {
        if let (Some(flat), Some(initial)) = (&self.flat, State::new(input)) {
            let result = machine::walk::walk(flat, initial, &self.setting.limit, choose);
            if result.cycle || result.overflow {
                return None;
            }
            return Some(Run {
                terminal: result.terminal.map(|state| state.observation()),
                work: result.work,
                depth: result.depth,
            });
        }
        let result = execution::walk(
            self.program,
            input,
            self.vocabulary,
            self.setting.admission,
            self.setting.bound,
            choose,
        );
        if result.cycle || result.overflow {
            return None;
        }
        Some(Run {
            terminal: result.terminal,
            work: result.work,
            depth: result.depth,
        })
    }

    fn cost(&self, input: &Configuration) -> Option<(usize, usize)> {
        if let (Some(flat), Some(initial)) = (&self.flat, State::new(input)) {
            let result = schedule(flat, initial, &self.setting.limit)?;
            return Some((result.work(), result.span()));
        }
        let result = self.run(input, |_| 0)?;
        Some((result.work, result.depth))
    }

    fn different(&self, output: &Observation, example: &Example) -> Outcome {
        let output = output.configuration();
        let distance = distance(&output, &example.output.configuration()).max(FLOOR);
        Outcome::Different { output, distance }
    }

    fn choice(&self, terminal: &[Observation], example: &Example) -> Outcome {
        terminal
            .iter()
            .map(|observation| {
                let output = observation.configuration();
                let distance = distance(&output, &example.output.configuration()).max(FLOOR);
                Outcome::Choice { output, distance }
            })
            .max_by(|left, right| left.distance().total_cmp(&right.distance()))
            .unwrap_or(Outcome::Divergent)
    }

    fn outcome(&self, example: &Example, index: usize, remaining: usize) -> (Outcome, usize) {
        let budget = self.setting.limit.individualization;
        let expected = example.output.key(budget);
        let trial = self.trial(&example.input, remaining, Some(&expected));
        {
            let state = trial.state;
            (self.judge(example, index, &expected, trial), state)
        }
    }

    fn judge(
        &self,
        example: &Example,
        index: usize,
        expected: &Key<Value>,
        trial: Trial,
    ) -> Outcome {
        let budget = self.setting.limit.individualization;
        if trial.cycle {
            return Outcome::Divergent;
        }
        if trial.terminal.len() > 1 {
            return self.choice(&trial.terminal, example);
        }
        if trial.truncated {
            return trial
                .terminal
                .first()
                .map_or(Outcome::Divergent, |terminal| {
                    self.different(terminal, example)
                });
        }
        if !trial.overflow {
            let Some(terminal) = trial.terminal.first() else {
                return Outcome::Divergent;
            };
            if terminal.key(budget) != *expected {
                return self.different(terminal, example);
            }
            return match self.cost(&example.input) {
                Some((work, span)) => Outcome::Exact {
                    work,
                    span,
                    verified: true,
                },
                None => Outcome::Divergent,
            };
        }
        self.sampled(example, index, expected)
    }

    fn sampled(&self, example: &Example, index: usize, expected: &Key<Value>) -> Outcome {
        let budget = self.setting.limit.individualization;
        let Some(first) = self.run(&example.input, |_| 0) else {
            return Outcome::Divergent;
        };
        let mut generator = Generator::new(value(&(self.program, index)));
        let mut terminal = vec![first.terminal.clone()];
        for _ in 0..self.setting.sample {
            let Some(run) = self.run(&example.input, |count| generator.below(count)) else {
                return Outcome::Divergent;
            };
            terminal.push(run.terminal);
        }
        let Some(observed) = terminal.into_iter().collect::<Option<Vec<_>>>() else {
            return Outcome::Divergent;
        };
        let mut distinct: Vec<Observation> = Vec::new();
        for observation in observed {
            if distinct
                .iter()
                .all(|known| known.key(budget) != observation.key(budget))
            {
                distinct.push(observation);
            }
        }
        match distinct.as_slice() {
            [single] if single.key(budget) == *expected => Outcome::Exact {
                work: first.work,
                span: first.depth,
                verified: false,
            },
            [single] => self.different(single, example),
            _ => self.choice(&distinct, example),
        }
    }
}

pub fn evaluate(
    program: &Program,
    example: &[Example],
    vocabulary: &Vocabulary,
    setting: &Setting,
) -> Evaluation {
    let context = Context {
        program,
        flat: Flat::new(program),
        vocabulary,
        setting,
    };
    let mut remaining = setting.budget;
    let outcome = example
        .iter()
        .enumerate()
        .map(|(index, example)| {
            if remaining == 0 {
                return Outcome::Divergent;
            }
            let (outcome, state) = context.outcome(example, index, remaining);
            remaining = remaining.saturating_sub(state);
            outcome
        })
        .collect::<Vec<_>>();
    let count = outcome.len().max(1) as f64;
    let correctness = outcome
        .iter()
        .map(|outcome| 1.0 - outcome.distance())
        .sum::<f64>()
        / count;
    let correct = outcome
        .iter()
        .all(|outcome| matches!(outcome, Outcome::Exact { .. }));
    let verified = outcome
        .iter()
        .all(|outcome| matches!(outcome, Outcome::Exact { verified: true, .. }));
    let (work, span) = outcome
        .iter()
        .fold((0.0, 0.0), |(work, span), outcome| match outcome {
            Outcome::Exact {
                work: effort,
                span: depth,
                ..
            } => (work + *effort as f64, span + *depth as f64),
            _ => (work, span),
        });
    let time = outcome
        .iter()
        .map(|outcome| outcome.time(setting.processor))
        .sum::<f64>()
        / count;
    let size = Size::new(program);
    Evaluation {
        outcome,
        correctness,
        correct,
        verified,
        time,
        work: work / count,
        span: span / count,
        size,
        cost: time + setting.size * size.total() as f64,
    }
}

pub fn behavior(
    program: &Program,
    input: &Configuration,
    vocabulary: &Vocabulary,
    setting: &Setting,
) -> Option<Observation> {
    let context = Context {
        program,
        flat: Flat::new(program),
        vocabulary,
        setting,
    };
    let trial = context.trial(input, setting.budget, None);
    if trial.cycle || trial.truncated || trial.terminal.len() > 1 {
        return None;
    }
    if !trial.overflow {
        return trial.terminal.into_iter().next();
    }
    let reference = context.run(input, |_| 0)?.terminal?;
    let example = Example {
        input: input.clone(),
        output: reference.clone(),
    };
    let expected = reference.key(setting.limit.individualization);
    matches!(
        context.sampled(&example, usize::MAX, &expected),
        Outcome::Exact { .. }
    )
    .then_some(reference)
}

fn changed<'example>(
    example: &'example [Example],
    vocabulary: &Vocabulary,
    setting: &Setting,
) -> Vec<&'example Example> {
    let empty = evaluate(&Program::default(), example, vocabulary, setting);
    example
        .iter()
        .zip(&empty.outcome)
        .filter(|(_, outcome)| !matches!(outcome, Outcome::Exact { .. }))
        .map(|(example, _)| example)
        .collect()
}

pub fn floor(example: &[Example], vocabulary: &Vocabulary, setting: &Setting) -> f64 {
    let changed = changed(example, vocabulary, setting).len();
    changed as f64 * (1.0 + 1.0 / setting.processor) / example.len().max(1) as f64
}

pub fn memorization(example: &[Example], vocabulary: &Vocabulary, setting: &Setting) -> f64 {
    let table = Program::from(
        changed(example, vocabulary, setting)
            .into_iter()
            .map(|example| {
                Rule::new(
                    example.input.coherence().to_vec(),
                    example
                        .output
                        .configuration()
                        .coherence()
                        .iter()
                        .cloned()
                        .map(Output::plain)
                        .collect(),
                )
            })
            .collect::<Vec<_>>(),
    );
    floor(example, vocabulary, setting) + setting.size * Size::new(&table).total() as f64
}

pub fn potential(evaluation: &Evaluation, baseline: f64) -> f64 {
    if !evaluation.correct {
        return evaluation.correctness.min(0.999);
    }
    2.0 + ((baseline - evaluation.cost) / baseline).clamp(-0.99, 1.0)
}
