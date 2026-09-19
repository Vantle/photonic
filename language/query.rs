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
            .map(|plan| 1 + plan.input.len() + plan.input.iter().map(Vec::len).sum::<usize>())
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
        let key = Key {
            pattern: self.rule[rule],
            frame,
            owner,
        };
        let plan = &self.plan[key.pattern];
        let previous = self.entry.get(&key);
        let selection = if let Some(previous) =
            previous.filter(|previous| previous.revision == plan.revision)
        {
            self.reuse += 1;
            previous.selection.clone()
        } else {
            self.preparation += 1;
            let pattern = if let Some(previous) = previous {
                previous.selection.pattern.clone()
            } else {
                Arc::new(if plan.input.is_empty() {
                    vec![Vec::new()]
                } else {
                    matching::pattern(&plan.input, Some(owner))
                })
            };
            Arc::new(Selection::new(pattern, index, frame))
        };
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
