use crate::plan::Input;
use crate::program::Program;
use std::collections::HashMap;

pub(crate) struct Catalog {
    input: Vec<Input>,
    rule: Vec<usize>,
    member: Vec<Vec<usize>>,
    retained: usize,
}

impl Catalog {
    pub fn new(program: &Program) -> Self {
        let mut identity = HashMap::new();
        let mut fragment = HashMap::new();
        let mut input = Vec::new();
        let rule = program
            .rule
            .iter()
            .map(|rule| {
                *identity.entry(&rule.input).or_insert_with(|| {
                    let index = input.len();
                    input.push(Input::shared(&rule.input, &mut fragment));
                    index
                })
            })
            .collect::<Vec<_>>();
        let mut member = vec![Vec::new(); input.len()];
        for (index, &input) in rule.iter().enumerate() {
            member[input].push(index);
        }
        let retained =
            input.iter().map(Input::retained).sum::<usize>() + rule.len() * 2 + member.len();
        Self {
            input,
            rule,
            member,
            retained,
        }
    }

    pub fn count(&self) -> usize {
        self.input.len()
    }

    pub fn rule(&self, rule: usize) -> usize {
        self.rule[rule]
    }

    pub fn member(&self, input: usize) -> &[usize] {
        &self.member[input]
    }

    pub fn input(&self, index: usize) -> &Input {
        &self.input[index]
    }

    pub fn retained(&self) -> usize {
        self.retained
    }
}
