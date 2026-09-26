mod sequence;

pub(crate) use sequence::Sequence;

use crate::profile;
use crate::program::{Program, Symbol};
use crate::snapshot::{Definition, Frame, Node, Token, Value, World};
use crate::state::State;
use crate::status::Status;
use std::collections::HashMap;
use std::sync::Arc;

pub(crate) struct Builder<'program> {
    program: &'program Program,
    atom: HashMap<usize, Arc<str>, crate::hashing::Builder>,
    scope: HashMap<usize, Arc<str>, crate::hashing::Builder>,
}

impl<'program> Builder<'program> {
    pub(crate) fn new(program: &'program Program) -> Self {
        Self {
            program,
            atom: HashMap::default(),
            scope: HashMap::default(),
        }
    }

    pub(crate) fn definition(&self) -> Vec<Definition> {
        (0..self.program.rule.len())
            .map(|index| Definition {
                name: self.program.rule[index].name.clone(),
                rule: self.program.definition(index),
            })
            .collect()
    }

    fn particle<'token>(
        &mut self,
        value: impl IntoIterator<Item = &'token crate::state::Token>,
    ) -> Vec<Token> {
        value
            .into_iter()
            .map(|token| Token {
                id: token.id,
                value: match token.value {
                    Symbol::Atom(index) => Value::Atom(
                        self.atom
                            .entry(index)
                            .or_insert_with(|| Arc::from(self.program.atom[index].as_str()))
                            .clone(),
                    ),
                    Symbol::Rule(index) => Value::Rule(index),
                },
                capture: token.capture,
            })
            .collect()
    }

    pub(crate) fn node(&mut self, id: usize, state: &State, status: Status) -> Node {
        let _scope = profile::Scope::new(profile::Phase::Rendering);
        Node {
            id,
            world: state
                .world
                .iter()
                .map(|world| World {
                    frame: world.frame,
                    particle: self.particle(&world.particle),
                })
                .collect(),
            frame: state
                .frame
                .iter()
                .map(|frame| {
                    let scope = self
                        .scope
                        .entry(frame.scope)
                        .or_insert_with(|| Arc::from(self.program.scope[frame.scope].name.as_str()))
                        .clone();
                    Frame {
                        scope,
                        opener: self.program.scope[frame.scope].opener,
                        parent: frame.parent,
                        lexical: frame.lexical,
                        particle: self.particle(&frame.particle),
                        held: self.particle(&frame.held),
                    }
                })
                .collect(),
            status,
        }
    }
}

#[cfg(test)]
#[path = "test/render.rs"]
mod test;
