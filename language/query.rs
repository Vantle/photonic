use crate::index::Index;
use crate::matching;
use crate::program::{Program, Symbol};
use crate::selection::Selection;
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

#[derive(Eq, Hash, PartialEq)]
struct Key {
    pattern: usize,
    frame: usize,
    owner: usize,
}

struct Plan {
    input: Vec<Vec<Symbol>>,
    revision: usize,
    pattern: Arc<Vec<Vec<matching::Term>>>,
    capture: bool,
}

struct Entry {
    selection: Arc<Selection>,
    generation: usize,
    revision: usize,
}

pub(crate) struct Query {
    plan: Vec<Plan>,
    rule: Vec<usize>,
    trigger: HashMap<Symbol, Vec<usize>>,
    empty: Vec<usize>,
    entry: HashMap<Key, Entry>,
    generation: usize,
    retained: usize,
    base: usize,
    pub preparation: usize,
    pub reuse: usize,
}

impl Query {
    pub(crate) fn new(program: &Program) -> Self {
        let mut catalog = HashMap::new();
        let mut plan = Vec::new();
        let rule = program
            .rule
            .iter()
            .map(|rule| {
                *catalog.entry(&rule.input).or_insert_with(|| {
                    let index = plan.len();
                    plan.push(Plan {
                        input: rule.input.clone(),
                        revision: 0,
                        pattern: Arc::new(if rule.input.is_empty() {
                            vec![Vec::new()]
                        } else {
                            matching::pattern(&rule.input, None)
                        }),
                        capture: rule
                            .input
                            .iter()
                            .flatten()
                            .any(|symbol| matches!(symbol, Symbol::Rule(_))),
                    });
                    index
                })
            })
            .collect::<Vec<_>>();
        let mut trigger: HashMap<_, Vec<_>> = HashMap::new();
        let mut empty = Vec::new();
        for (index, plan) in plan.iter().enumerate() {
            if plan.input.is_empty() || plan.input.iter().any(Vec::is_empty) {
                empty.push(index);
            }
            for symbol in plan
                .input
                .iter()
                .flatten()
                .copied()
                .collect::<BTreeSet<_>>()
            {
                trigger.entry(symbol).or_default().push(index);
            }
        }
        let base = plan
            .iter()
            .map(|plan| {
                1 + plan.input.len() * 2 + plan.input.iter().map(Vec::len).sum::<usize>() * 2
            })
            .sum::<usize>()
            + rule.len()
            + empty.len()
            + trigger.len()
            + trigger.values().map(Vec::len).sum::<usize>();
        Self {
            plan,
            rule,
            trigger,
            empty,
            entry: HashMap::new(),
            generation: 0,
            retained: 0,
            base,
            preparation: 0,
            reuse: 0,
        }
    }

    pub(crate) fn select(
        &mut self,
        rule: usize,
        frame: usize,
        owner: usize,
        index: &Index,
    ) -> Arc<Selection> {
        let plan = &self.plan[self.rule[rule]];
        let key = Key {
            pattern: self.rule[rule],
            frame,
            owner: if plan.capture { owner } else { 0 },
        };
        if let Some(entry) = self.entry.get_mut(&key) {
            let selection = if entry.revision == plan.revision {
                None
            } else if entry.generation + 1 == self.generation {
                entry.selection.advance(index, frame)
            } else {
                Some(Selection::new(
                    entry.selection.pattern.clone(),
                    index,
                    frame,
                ))
            };
            if let Some(selection) = selection {
                self.preparation += 1;
                entry.selection = Arc::new(selection);
            } else {
                self.reuse += 1;
            }
            entry.generation = self.generation;
            entry.revision = plan.revision;
            return entry.selection.clone();
        }
        self.preparation += 1;
        let pattern = if plan.capture {
            Arc::new(matching::pattern(&plan.input, Some(owner)))
        } else {
            plan.pattern.clone()
        };
        let selection = Arc::new(Selection::new(pattern, index, frame));
        self.entry.insert(
            key,
            Entry {
                selection: selection.clone(),
                generation: self.generation,
                revision: plan.revision,
            },
        );
        selection
    }

    pub(crate) fn advance(&mut self, index: &Index) {
        self.generation += 1;
        for symbol in &index.altered {
            for &plan in self.trigger.get(symbol).into_iter().flatten() {
                self.plan[plan].revision += 1;
            }
        }
        if index.occupied {
            for &plan in &self.empty {
                self.plan[plan].revision += 1;
            }
        }
    }

    pub(crate) fn finish(&mut self) {
        self.entry
            .retain(|_, entry| entry.generation == self.generation);
        self.retained = self
            .entry
            .values()
            .map(|entry| 1 + entry.selection.retained())
            .sum();
    }

    pub(crate) fn retained(&self) -> usize {
        self.base + self.retained
    }
}

#[cfg(test)]
#[path = "test/query.rs"]
mod test;
