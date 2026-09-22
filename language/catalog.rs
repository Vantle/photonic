use crate::plan::Input;
use crate::program::Program;
use std::collections::HashMap;

mod scope;

pub(crate) struct Location {
    pub scope: usize,
    pub position: usize,
}

pub(crate) struct Catalog {
    input: Vec<Input>,
    rule: Vec<usize>,
    retained: usize,
    scope: Vec<scope::Scope>,
    owner: Vec<Vec<Location>>,
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
        let scope = program
            .scope
            .iter()
            .map(|scope| scope::Scope::new(&scope.rule, &rule))
            .collect::<Vec<_>>();
        let mut owner = (0..input.len()).map(|_| Vec::new()).collect::<Vec<_>>();
        for (index, scope) in scope.iter().enumerate() {
            for (position, input) in scope.iter().enumerate() {
                owner[input].push(Location {
                    scope: index,
                    position,
                });
            }
        }
        let retained = input.iter().map(Input::retained).sum::<usize>() + rule.len();
        let retained = retained
            + owner.iter().map(Vec::len).sum::<usize>()
            + scope.iter().map(scope::Scope::retained).sum::<usize>();
        Self {
            input,
            rule,
            retained,
            scope,
            owner,
        }
    }

    pub fn owner(&self, input: usize) -> &[Location] {
        &self.owner[input]
    }

    pub fn count(&self) -> usize {
        self.input.len()
    }

    pub fn scope(&self, scope: usize, position: usize) -> (usize, &[usize]) {
        self.scope[scope].get(position)
    }

    pub fn width(&self, scope: usize) -> usize {
        self.scope[scope].len()
    }

    pub fn position(&self, scope: usize, input: usize) -> Option<usize> {
        self.scope[scope].position(input)
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

#[cfg(test)]
#[path = "test/catalog.rs"]
mod test;
