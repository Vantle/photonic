use crate::plan::Input;
use crate::program::{Program, Symbol};
use std::collections::{BTreeSet, HashMap};

pub(crate) struct Catalog {
    input: Vec<Input>,
    rule: Vec<usize>,
    revision: Vec<usize>,
    trigger: HashMap<Symbol, Vec<usize>>,
    empty: Vec<usize>,
    retained: usize,
}

impl Catalog {
    pub fn new(program: &Program) -> Self {
        let mut identity = HashMap::new();
        let mut input = Vec::new();
        let rule = program
            .rule
            .iter()
            .map(|rule| {
                *identity.entry(&rule.input).or_insert_with(|| {
                    let index = input.len();
                    input.push(Input::new(&rule.input));
                    index
                })
            })
            .collect::<Vec<_>>();
        let mut trigger: HashMap<_, Vec<_>> = HashMap::new();
        let mut empty = Vec::new();
        for (index, input) in input.iter().enumerate() {
            if input.empty() {
                empty.push(index);
            }
            for symbol in input.symbol().collect::<BTreeSet<_>>() {
                trigger.entry(symbol).or_default().push(index);
            }
        }
        let revision = vec![0; input.len()];
        let retained = input.iter().map(Input::retained).sum::<usize>()
            + revision.len()
            + rule.len()
            + empty.len()
            + trigger.len()
            + trigger.values().map(Vec::len).sum::<usize>();
        Self {
            input,
            rule,
            revision,
            trigger,
            empty,
            retained,
        }
    }

    pub fn rule(&self, rule: usize) -> usize {
        self.rule[rule]
    }

    pub fn input(&self, index: usize) -> &Input {
        &self.input[index]
    }

    pub fn revision(&self, index: usize) -> usize {
        self.revision[index]
    }

    pub fn advance(&mut self, change: impl IntoIterator<Item = Symbol>, occupied: bool) {
        for symbol in change {
            for &input in self.trigger.get(&symbol).into_iter().flatten() {
                self.revision[input] += 1;
            }
        }
        if occupied {
            for &input in &self.empty {
                self.revision[input] += 1;
            }
        }
    }

    pub fn retained(&self) -> usize {
        self.retained
    }
}
