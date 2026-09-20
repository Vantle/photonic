use crate::plan::Input;
use crate::program::Program;
use std::collections::HashMap;

pub(crate) struct Catalog {
    input: Vec<Input>,
    rule: Vec<usize>,
    retained: usize,
    scope: Vec<HashMap<usize, Vec<usize>>>,
    owner: Vec<Vec<usize>>,
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
        let scope = program
            .scope
            .iter()
            .map(|scope| {
                let mut group: HashMap<usize, Vec<usize>> = HashMap::new();
                for &index in &scope.rule {
                    group.entry(rule[index]).or_default().push(index);
                }
                group
            })
            .collect::<Vec<_>>();
        let mut owner = vec![Vec::new(); input.len()];
        for (index, scope) in scope.iter().enumerate() {
            for &input in scope.keys() {
                owner[input].push(index);
            }
        }
        let retained = input.iter().map(Input::retained).sum::<usize>() + rule.len();
        let retained = retained
            + owner.iter().map(Vec::len).sum::<usize>()
            + scope
                .iter()
                .map(|scope| scope.len() + scope.values().map(Vec::len).sum::<usize>())
                .sum::<usize>();
        Self {
            input,
            rule,
            retained,
            scope,
            owner,
        }
    }

    pub fn owner(&self, input: usize) -> &[usize] {
        &self.owner[input]
    }

    pub fn count(&self) -> usize {
        self.input.len()
    }

    pub fn scope(&self, scope: usize, input: usize) -> &[usize] {
        self.scope[scope].get(&input).map_or(&[], Vec::as_slice)
    }

    pub fn rule(&self, rule: usize) -> usize {
        self.rule[rule]
    }

    pub fn input(&self, index: usize) -> &Input {
        &self.input[index]
    }

    pub fn retained(&self) -> usize {
        self.retained
    }
}
