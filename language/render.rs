mod sequence;

pub(crate) use sequence::Sequence;

use crate::profile;
use crate::program::{Program, Symbol};
use crate::snapshot::{Frame, Node, Token, World};
use crate::state::State;
use crate::status::Status;
use std::collections::HashMap;
use std::sync::Arc;

struct Text {
    label: Arc<str>,
    display: Arc<str>,
}

pub(crate) struct Builder<'program> {
    program: &'program Program,
    text: HashMap<Symbol, Text, crate::hashing::Builder>,
    scope: HashMap<usize, Arc<str>, crate::hashing::Builder>,
}

impl<'program> Builder<'program> {
    pub(crate) fn new(program: &'program Program) -> Self {
        Self {
            program,
            text: HashMap::default(),
            scope: HashMap::default(),
        }
    }

    pub(crate) fn definition(&self) -> Vec<crate::snapshot::Definition> {
        (0..self.program.rule.len())
            .map(|index| crate::snapshot::Definition {
                label: format!("§{index}"),
                display: self.program.label(Symbol::Rule(index)),
            })
            .collect()
    }

    fn particle<'token>(
        &mut self,
        value: impl IntoIterator<Item = &'token crate::state::Token>,
    ) -> Vec<Token> {
        value
            .into_iter()
            .map(|token| {
                let text = self
                    .text
                    .entry(token.value)
                    .or_insert_with(|| match token.value {
                        Symbol::Atom(index) => {
                            let label = Arc::<str>::from(self.program.atom[index].as_str());
                            Text {
                                display: label.clone(),
                                label,
                            }
                        }
                        Symbol::Rule(index) => Text {
                            label: format!("§{index}").into(),
                            display: self.program.label(token.value).into(),
                        },
                    });
                Token {
                    id: token.id,
                    label: text.label.clone(),
                    display: text.display.clone(),
                    capture: token.capture,
                }
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
