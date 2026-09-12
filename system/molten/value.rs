use crate::program::{Instruction, Output, Symbol};
use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use std::task::Poll;

#[derive(Clone, Eq, PartialEq)]
enum Shape {
    Value(Symbol),
    Particle(Vec<Symbol>),
    Rule(Arc<Instruction>),
    Output(Output),
}

#[derive(Clone)]
enum Obligation {
    Pair(Shape, Shape),
    Set {
        pattern: Arc<Vec<Shape>>,
        actual: Arc<Vec<Shape>>,
        selected: Vec<usize>,
        cursor: usize,
    },
}

#[derive(Clone)]
struct Branch {
    binding: BTreeMap<String, Symbol>,
    obligation: Vec<Obligation>,
}

pub struct Search {
    branch: Vec<Branch>,
    seen: HashSet<BTreeMap<String, Symbol>>,
}

fn concrete(value: &Symbol) -> bool {
    match value {
        Symbol::Variable(_) => false,
        Symbol::Structure(_, particle) => particle.iter().all(concrete),
        Symbol::Atom(_) | Symbol::Rule(_, _) => true,
    }
}

impl Branch {
    fn set(&mut self, pattern: Vec<Shape>, actual: Vec<Shape>) -> bool {
        if pattern.len() != actual.len() {
            return false;
        }
        self.obligation.push(Obligation::Set {
            pattern: Arc::new(pattern),
            actual: Arc::new(actual),
            selected: Vec::new(),
            cursor: 0,
        });
        true
    }

    fn particle(&mut self, pattern: Vec<Symbol>, actual: Vec<Symbol>) -> bool {
        self.set(
            pattern.into_iter().map(Shape::Value).collect(),
            actual.into_iter().map(Shape::Value).collect(),
        )
    }

    fn input(&mut self, pattern: &[Vec<Symbol>], actual: &[Vec<Symbol>]) -> bool {
        self.set(
            pattern.iter().cloned().map(Shape::Particle).collect(),
            actual.iter().cloned().map(Shape::Particle).collect(),
        )
    }

    fn pair(&mut self, pattern: Shape, actual: Shape) -> bool {
        match (pattern, actual) {
            (Shape::Value(Symbol::Variable(name)), Shape::Value(actual)) => {
                if !concrete(&actual) {
                    return false;
                }
                if let Some(previous) = self.binding.get(&name) {
                    return previous == &actual;
                }
                self.binding.insert(name, actual);
                true
            }
            (Shape::Value(Symbol::Atom(left)), Shape::Value(Symbol::Atom(right))) => left == right,
            (
                Shape::Value(Symbol::Structure(left, pattern)),
                Shape::Value(Symbol::Structure(right, actual)),
            ) => left == right && self.particle(pattern, actual),
            (
                Shape::Value(Symbol::Rule(pattern, _)),
                Shape::Value(Symbol::Rule(actual, capture)),
            ) => {
                let actual = capture.map_or(actual.clone(), |frame| Arc::new(actual.close(frame)));
                self.obligation
                    .push(Obligation::Pair(Shape::Rule(pattern), Shape::Rule(actual)));
                true
            }
            (Shape::Particle(pattern), Shape::Particle(actual)) => self.particle(pattern, actual),
            (Shape::Rule(pattern), Shape::Rule(actual)) => {
                if !self.set(
                    pattern.output.iter().cloned().map(Shape::Output).collect(),
                    actual.output.iter().cloned().map(Shape::Output).collect(),
                ) {
                    return false;
                }
                match (&pattern.negative, &actual.negative) {
                    (None, None) => {}
                    (Some(pattern), Some(actual)) => {
                        if !self.input(pattern, actual) {
                            return false;
                        }
                    }
                    _ => return false,
                }
                self.input(&pattern.input, &actual.input)
            }
            (Shape::Output(pattern), Shape::Output(actual)) => {
                match (pattern.body, actual.body) {
                    (None, None) => {}
                    (Some(pattern), Some(actual)) => {
                        if !self.set(
                            pattern.rule.iter().cloned().map(Shape::Rule).collect(),
                            actual.rule.iter().cloned().map(Shape::Rule).collect(),
                        ) {
                            return false;
                        }
                    }
                    _ => return false,
                }
                self.particle(pattern.particle, actual.particle)
            }
            _ => false,
        }
    }
}

impl Search {
    pub fn new(pattern: Symbol, actual: Symbol) -> Self {
        Self {
            branch: vec![Branch {
                binding: BTreeMap::new(),
                obligation: vec![Obligation::Pair(
                    Shape::Value(pattern),
                    Shape::Value(actual),
                )],
            }],
            seen: HashSet::new(),
        }
    }

    pub fn retained(&self) -> usize {
        self.seen.len()
            + self
                .branch
                .iter()
                .map(|branch| 1 + branch.binding.len() + branch.obligation.len())
                .sum::<usize>()
    }

    pub fn step(&mut self) -> Poll<Option<BTreeMap<String, Symbol>>> {
        let Some(mut branch) = self.branch.pop() else {
            return Poll::Ready(None);
        };
        let Some(obligation) = branch.obligation.pop() else {
            return if self.seen.insert(branch.binding.clone()) {
                Poll::Ready(Some(branch.binding))
            } else {
                Poll::Pending
            };
        };
        match obligation {
            Obligation::Pair(pattern, actual) => {
                if branch.pair(pattern, actual) {
                    if branch.obligation.is_empty() {
                        return if self.seen.insert(branch.binding.clone()) {
                            Poll::Ready(Some(branch.binding))
                        } else {
                            Poll::Pending
                        };
                    }
                    self.branch.push(branch);
                }
            }
            Obligation::Set {
                pattern,
                actual,
                selected,
                cursor,
            } => {
                if selected.len() == pattern.len() {
                    self.branch.push(branch);
                    return Poll::Pending;
                }
                if cursor == actual.len() {
                    return Poll::Pending;
                }
                let mut alternative = branch.clone();
                alternative.obligation.push(Obligation::Set {
                    pattern: pattern.clone(),
                    actual: actual.clone(),
                    selected: selected.clone(),
                    cursor: cursor + 1,
                });
                self.branch.push(alternative);
                if selected.contains(&cursor)
                    || (0..cursor)
                        .any(|index| !selected.contains(&index) && actual[index] == actual[cursor])
                {
                    return Poll::Pending;
                }
                let pair =
                    Obligation::Pair(pattern[selected.len()].clone(), actual[cursor].clone());
                let mut selected = selected;
                selected.push(cursor);
                branch.obligation.push(Obligation::Set {
                    pattern,
                    actual,
                    selected,
                    cursor: 0,
                });
                branch.obligation.push(pair);
                self.branch.push(branch);
            }
        }
        Poll::Pending
    }
}

pub fn matching(pattern: &Symbol, actual: &Symbol) -> Vec<BTreeMap<String, Symbol>> {
    let mut search = Search::new(pattern.clone(), actual.clone());
    let mut result = Vec::new();
    loop {
        match search.step() {
            Poll::Pending => {}
            Poll::Ready(Some(binding)) => result.push(binding),
            Poll::Ready(None) => return result,
        }
    }
}
