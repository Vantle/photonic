use crate::program::{Program, Symbol};
use std::collections::{BTreeSet, HashMap};

pub(crate) struct Network {
    pub enabled: BTreeSet<usize>,
    pub scope: Vec<Option<usize>>,
    missing: Vec<usize>,
    trigger: HashMap<Symbol, Vec<usize>>,
    retained: usize,
}

impl Network {
    pub(crate) fn new(program: &Program) -> Self {
        let mut network = Self {
            enabled: BTreeSet::new(),
            scope: vec![None; program.rule.len()],
            missing: Vec::with_capacity(program.rule.len()),
            trigger: HashMap::new(),
            retained: 0,
        };
        for (index, rule) in program.rule.iter().enumerate() {
            let input = rule
                .input
                .iter()
                .flatten()
                .copied()
                .collect::<BTreeSet<_>>();
            network.missing.push(input.len());
            if input.is_empty() {
                network.enabled.insert(index);
            }
            for symbol in input {
                network.trigger.entry(symbol).or_default().push(index);
            }
        }
        for (index, scope) in program.scope.iter().enumerate() {
            for &rule in &scope.rule {
                network.scope[rule] = Some(index);
            }
        }
        network.retained = network.scope.len()
            + network.missing.len()
            + network.trigger.len()
            + network.trigger.values().map(Vec::len).sum::<usize>();
        network
    }

    pub(crate) fn change(&mut self, symbol: Symbol, present: bool) {
        for &rule in self.trigger.get(&symbol).into_iter().flatten() {
            if present {
                self.missing[rule] -= 1;
                if self.missing[rule] == 0 {
                    self.enabled.insert(rule);
                }
            } else {
                self.missing[rule] += 1;
                self.enabled.remove(&rule);
            }
        }
    }

    pub(crate) fn retained(&self) -> usize {
        self.retained + self.enabled.len()
    }
}
