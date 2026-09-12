use crate::program::Symbol;
use crate::state::{State, Token, World};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Environment {
    state: State,
    anchor: Vec<Option<usize>>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Constraint {
    name: Vec<String>,
    environment: Environment,
    mapping: Vec<Option<usize>>,
}

pub fn normalize(state: State, mapping: &[Option<usize>]) -> Environment {
    let mut search =
        crate::canonical::Search::anchored(std::sync::Arc::new(state), mapping.to_vec());
    while !search.step() {}
    let canonical = search.finish().unwrap();
    let mut anchor = vec![None; canonical.state.frame.len()];
    for (index, target) in canonical.frame.iter().enumerate() {
        if let Some(target) = target {
            anchor[*target] = mapping.get(index).copied().flatten();
        }
    }
    Environment {
        state: canonical.state,
        anchor,
    }
}

pub fn environment(
    state: &State,
    binding: &BTreeMap<String, Symbol>,
    mapping: &[Option<usize>],
) -> Environment {
    normalize(
        State {
            world: vec![World {
                frame: 0,
                particle: binding
                    .values()
                    .enumerate()
                    .map(|(id, value)| Token {
                        id,
                        value: Symbol::Structure(id, vec![value.clone()]),
                    })
                    .collect(),
            }],
            frame: state.frame.clone(),
        },
        mapping,
    )
}

impl Constraint {
    pub fn new(
        state: &State,
        binding: BTreeMap<String, Symbol>,
        mapping: &[Option<usize>],
    ) -> Self {
        Self {
            name: binding.keys().cloned().collect(),
            environment: environment(state, &binding, mapping),
            mapping: mapping.to_vec(),
        }
    }

    pub fn project(&self, mapping: &[Option<usize>]) -> Self {
        Self {
            mapping: mapping.to_vec(),
            ..self.clone()
        }
    }

    pub fn accepts(&self, state: &State, binding: &BTreeMap<String, Symbol>) -> bool {
        let Some(binding) = self
            .name
            .iter()
            .map(|name| Some((name.clone(), binding.get(name)?.clone())))
            .collect::<Option<BTreeMap<_, _>>>()
        else {
            return false;
        };
        environment(state, &binding, &self.mapping) == self.environment
    }
}

pub fn contains(value: &Symbol, name: &str) -> bool {
    match value {
        Symbol::Atom(_) => false,
        Symbol::Variable(variable) => variable == name,
        Symbol::Structure(_, particle) => particle.iter().any(|value| contains(value, name)),
        Symbol::Rule(rule, _) => instruction(rule, name),
    }
}

fn instruction(rule: &crate::program::Instruction, name: &str) -> bool {
    rule.input
        .iter()
        .flatten()
        .chain(rule.negative.iter().flatten().flatten())
        .chain(rule.output.iter().flat_map(|output| &output.particle))
        .any(|value| contains(value, name))
        || rule
            .output
            .iter()
            .filter_map(|output| output.body.as_ref())
            .flat_map(|scope| &scope.rule)
            .any(|rule| instruction(rule, name))
}
