use crate::edit::{Action, Local, Side};
use crate::objective::{Evaluation, Outcome};
use crate::task::{Goal, Task};
use code::configuration::Configuration;
use code::output::Output;
use code::particle::Particle;
use code::program::Program;
use code::rule::Rule;
use code::tree::{Place, walk};
use code::value::Value;
use network::input::{Input, Pointer};
use random::Generator;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const ATOM: usize = 128;
pub const EXAMPLE: usize = 4;
pub const GROUP: usize = 16;
pub const RULE: usize = 64;
pub const DEPTH: usize = 4;
pub const DIMENSION: usize = 32;
const PROCESSOR: usize = 9;
const WEIGHT: usize = 11;

#[derive(Clone, Copy)]
enum Kind {
    Summary,
    Symbol,
    Coherence,
    Element,
    Rule,
    Particle,
    Occurrence,
    Append,
    Divergent,
    Choice,
}

#[derive(Clone, Copy)]
enum Role {
    Source = 1,
    Target,
    Result,
    Input,
    Output,
}

#[derive(Clone, Copy)]
enum Position {
    Program = 1,
    Value,
    Body,
}

#[derive(Clone, Copy)]
enum Status {
    Exact = 1,
    Different,
    Choice,
    Divergent,
    Correct,
    Incorrect,
}

#[derive(Clone, Copy)]
enum Unary {
    Stop,
    Remove,
    Close,
    Open,
    Duplicate,
    Delete,
    Create,
    Purge,
    Trim,
    Strip,
}

#[derive(Clone, Copy)]
enum Binary {
    Insert,
    Extend,
    Nest,
    Graft,
    Enclose,
    Detach,
    Release,
}

const FIELD: [usize; 12] = [
    10,
    ATOM + 1,
    6,
    EXAMPLE + 1,
    GROUP + 1,
    RULE + 1,
    RULE + 1,
    DEPTH + 2,
    4,
    7,
    PROCESSOR + 1,
    WEIGHT + 1,
];

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Shape {
    pub width: usize,
    pub depth: usize,
    pub head: usize,
    pub hidden: usize,
    pub key: usize,
}

impl Default for Shape {
    fn default() -> Self {
        Self {
            width: 256,
            depth: 6,
            head: 256 / DIMENSION,
            hidden: 1_024,
            key: DIMENSION,
        }
    }
}

impl Shape {
    pub fn architecture(&self) -> network::configuration::Configuration {
        network::configuration::Configuration {
            field: FIELD.to_vec(),
            width: self.width,
            depth: self.depth,
            head: self.head,
            hidden: self.hidden,
            unary: 10,
            binary: 7,
            key: self.key,
            judge: 2,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Permutation {
    atom: Vec<u16>,
    example: Vec<usize>,
    group: Vec<u16>,
    rule: Vec<u16>,
}

fn shuffled(generator: &mut Generator, count: usize) -> Vec<u16> {
    let mut value = (1..=count as u16).collect::<Vec<_>>();
    generator.shuffle(&mut value);
    value
}

impl Permutation {
    pub fn new(generator: &mut Generator, example: usize) -> Self {
        let mut order = (0..example).collect::<Vec<_>>();
        generator.shuffle(&mut order);
        Self {
            atom: shuffled(generator, ATOM),
            example: order,
            group: shuffled(generator, GROUP),
            rule: shuffled(generator, RULE),
        }
    }

    fn atom(&self, atom: code::atom::Atom) -> u16 {
        self.atom[atom.index().min(ATOM - 1)]
    }

    fn group(&self, index: usize) -> u16 {
        self.group[index.min(GROUP - 1)]
    }

    fn rule(&self, index: usize) -> u16 {
        self.rule[index.min(RULE - 1)]
    }
}

#[derive(Clone, Copy, Default)]
struct Token {
    kind: u16,
    atom: u16,
    role: u16,
    example: u16,
    group: u16,
    rule: u16,
    parent: u16,
    depth: u16,
    place: u16,
    status: u16,
    processor: u16,
    weight: u16,
}

fn depth(value: usize) -> u16 {
    (value.min(DEPTH) + 1) as u16
}

fn processor(goal: &Goal) -> u16 {
    goal.processor
        .max(1.0)
        .log2()
        .round()
        .min((PROCESSOR - 1) as f64) as u16
        + 1
}

fn weight(goal: &Goal) -> u16 {
    (goal.size.max(f64::MIN_POSITIVE).log2().round() + (WEIGHT - 1) as f64)
        .clamp(0.0, (WEIGHT - 1) as f64) as u16
        + 1
}

#[derive(Default)]
struct Writer {
    feature: Vec<u16>,
}

impl Writer {
    fn push(&mut self, token: Token) -> u32 {
        let index = (self.feature.len() / FIELD.len()) as u32;
        self.feature.extend([
            token.kind,
            token.atom,
            token.role,
            token.example,
            token.group,
            token.rule,
            token.parent,
            token.depth,
            token.place,
            token.status,
            token.processor,
            token.weight,
        ]);
        index
    }

    fn members(
        &mut self,
        particle: &Particle,
        template: Token,
        permutation: &Permutation,
        level: usize,
    ) {
        for value in particle.value() {
            match value {
                Value::Atom(atom) => {
                    self.push(Token {
                        kind: Kind::Element as u16,
                        atom: permutation.atom(*atom),
                        ..template
                    });
                }
                Value::Rule(rule) => self.data(rule, template, permutation, level + 1),
            }
        }
    }

    fn data(&mut self, rule: &Rule, template: Token, permutation: &Permutation, level: usize) {
        self.push(Token {
            kind: Kind::Rule as u16,
            depth: depth(level),
            place: Position::Value as u16,
            ..template
        });
        let particle = rule
            .input()
            .iter()
            .chain(rule.output().iter().map(Output::particle));
        for (index, particle) in particle.enumerate() {
            let inner = Token {
                group: permutation.group(index),
                depth: depth(level),
                ..template
            };
            self.push(Token {
                kind: Kind::Particle as u16,
                ..inner
            });
            self.members(particle, inner, permutation, level);
        }
    }

    fn configuration(
        &mut self,
        value: &Configuration,
        role: Role,
        template: Token,
        permutation: &Permutation,
    ) {
        for (index, coherence) in value.coherence().iter().enumerate() {
            let inner = Token {
                role: role as u16,
                group: permutation.group(index),
                ..template
            };
            self.push(Token {
                kind: Kind::Coherence as u16,
                ..inner
            });
            self.members(coherence, inner, permutation, 0);
        }
    }
}

fn status(outcome: &Outcome) -> Status {
    match outcome {
        Outcome::Exact { .. } => Status::Exact,
        Outcome::Different { .. } => Status::Different,
        Outcome::Choice { .. } => Status::Choice,
        Outcome::Divergent => Status::Divergent,
    }
}

fn shown(evaluation: &Evaluation, permutation: &Permutation) -> Vec<usize> {
    let order = permutation
        .example
        .iter()
        .copied()
        .filter(|&index| index < evaluation.outcome.len());
    let mut result = order
        .clone()
        .filter(|&index| !matches!(evaluation.outcome[index], Outcome::Exact { .. }))
        .collect::<Vec<_>>();
    result
        .extend(order.filter(|&index| matches!(evaluation.outcome[index], Outcome::Exact { .. })));
    result.truncate(EXAMPLE);
    result
}

#[derive(Default)]
struct Registry {
    rule: Vec<u32>,
    particle: HashMap<(usize, Side, usize), u32>,
    append: HashMap<(usize, Side), u32>,
    occurrence: HashMap<(usize, Side, usize, code::atom::Atom), u32>,
}

fn program(writer: &mut Writer, program: &Program, permutation: &Permutation) -> Registry {
    let mut registry = Registry::default();
    let node = walk(program);
    for (index, entry) in node.iter().enumerate() {
        let (parent, place) = match entry.place {
            Place::Program => (0, Position::Program),
            Place::Value { parent } => (permutation.rule(parent), Position::Value),
            Place::Body { parent } => (permutation.rule(parent), Position::Body),
        };
        let template = Token {
            rule: permutation.rule(index),
            parent,
            depth: depth(entry.depth),
            ..Token::default()
        };
        registry.rule.push(writer.push(Token {
            kind: Kind::Rule as u16,
            place: place as u16,
            ..template
        }));
        for side in [Side::Input, Side::Output] {
            let role = match side {
                Side::Input => Role::Input,
                Side::Output => Role::Output,
            } as u16;
            let particle = match side {
                Side::Input => entry.rule.input().iter().collect::<Vec<_>>(),
                Side::Output => entry.rule.output().iter().map(Output::particle).collect(),
            };
            for (position, value) in particle.iter().enumerate() {
                let group = permutation.group(position);
                registry.particle.insert(
                    (index, side, position),
                    writer.push(Token {
                        kind: Kind::Particle as u16,
                        role,
                        group,
                        ..template
                    }),
                );
                for atom in value.value().iter().filter_map(Value::atom) {
                    let token = writer.push(Token {
                        kind: Kind::Occurrence as u16,
                        atom: permutation.atom(atom),
                        role,
                        group,
                        ..template
                    });
                    registry
                        .occurrence
                        .entry((index, side, position, atom))
                        .or_insert(token);
                }
            }
            registry.append.insert(
                (index, side),
                writer.push(Token {
                    kind: Kind::Append as u16,
                    role,
                    ..template
                }),
            );
        }
    }
    registry
}

pub fn encode(
    task: &Task,
    program: &Program,
    evaluation: &Evaluation,
    action: &[Action],
    permutation: &Permutation,
) -> Input {
    let mut writer = Writer::default();
    let goal = task.goal.unwrap_or_default();
    writer.push(Token {
        kind: Kind::Summary as u16,
        status: (if evaluation.correct {
            Status::Correct
        } else {
            Status::Incorrect
        }) as u16,
        processor: processor(&goal),
        weight: weight(&goal),
        ..Token::default()
    });
    let symbol = task
        .vocabulary
        .atom()
        .map(|atom| {
            writer.push(Token {
                kind: Kind::Symbol as u16,
                atom: permutation.atom(atom),
                ..Token::default()
            })
        })
        .collect::<Vec<_>>();
    for (position, index) in shown(evaluation, permutation).into_iter().enumerate() {
        let example = &task.example[index];
        let outcome = &evaluation.outcome[index];
        let template = Token {
            example: position as u16 + 1,
            status: status(outcome) as u16,
            ..Token::default()
        };
        writer.configuration(&example.input, Role::Source, template, permutation);
        writer.configuration(
            &example.output.configuration(),
            Role::Target,
            template,
            permutation,
        );
        match outcome {
            Outcome::Exact { .. } => {}
            Outcome::Different { output, .. } => {
                writer.configuration(output, Role::Result, template, permutation);
            }
            Outcome::Choice { output, .. } => {
                writer.push(Token {
                    kind: Kind::Choice as u16,
                    ..template
                });
                writer.configuration(output, Role::Result, template, permutation);
            }
            Outcome::Divergent => {
                writer.push(Token {
                    kind: Kind::Divergent as u16,
                    ..template
                });
            }
        }
    }
    let registry = self::program(&mut writer, program, permutation);
    let unary = |head: Unary, token: u32| Pointer::Unary {
        head: head as u16,
        token,
    };
    let binary = |head: Binary, left: u32, right: u32| Pointer::Binary {
        head: head as u16,
        left,
        right,
    };
    let pointer = action
        .iter()
        .map(|action| match *action {
            Action::Stop => unary(Unary::Stop, 0),
            Action::Create { atom } => unary(Unary::Create, symbol[atom.index()]),
            Action::Duplicate { rule } => unary(Unary::Duplicate, registry.rule[rule]),
            Action::Delete { rule } => unary(Unary::Delete, registry.rule[rule]),
            Action::Open { rule, side } => unary(Unary::Open, registry.append[&(rule, side)]),
            Action::Close {
                rule,
                side,
                particle,
            } => unary(Unary::Close, registry.particle[&(rule, side, particle)]),
            Action::Remove {
                rule,
                side,
                particle,
                atom,
            } => unary(
                Unary::Remove,
                registry.occurrence[&(rule, side, particle, atom)],
            ),
            Action::Insert {
                rule,
                side,
                particle,
                atom,
            } => match registry.particle.get(&(rule, side, particle)) {
                Some(&left) => binary(Binary::Insert, left, symbol[atom.index()]),
                None => binary(
                    Binary::Extend,
                    registry.append[&(rule, side)],
                    symbol[atom.index()],
                ),
            },
            Action::Nest {
                rule,
                side,
                particle,
                atom,
            } => match registry.particle.get(&(rule, side, particle)) {
                Some(&left) => binary(Binary::Nest, left, symbol[atom.index()]),
                None => binary(
                    Binary::Graft,
                    registry.append[&(rule, side)],
                    symbol[atom.index()],
                ),
            },
            Action::Enclose { rule, output, atom } => binary(
                Binary::Enclose,
                registry.particle[&(rule, Side::Output, output)],
                symbol[atom.index()],
            ),
            Action::Detach { rule, atom } => {
                binary(Binary::Detach, registry.rule[rule], symbol[atom.index()])
            }
            Action::Analogy(Local::Delete { rule }) => unary(Unary::Purge, registry.rule[rule]),
            Action::Analogy(Local::Close {
                rule,
                side,
                particle,
            }) => unary(Unary::Trim, registry.particle[&(rule, side, particle)]),
            Action::Analogy(Local::Remove {
                rule,
                side,
                particle,
                atom,
            }) => unary(
                Unary::Strip,
                registry.occurrence[&(rule, side, particle, atom)],
            ),
            Action::Analogy(Local::Detach { rule, atom }) => {
                binary(Binary::Release, registry.rule[rule], symbol[atom.index()])
            }
        })
        .collect();
    Input {
        feature: writer.feature,
        pointer,
    }
}
