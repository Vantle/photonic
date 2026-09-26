use photonic::snapshot::{Frame, Node, Token, Value};
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum Occurrence {
    Atom {
        id: usize,
        label: Arc<str>,
    },
    Rule {
        id: usize,
        rule: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        capture: Option<usize>,
    },
}

#[derive(Serialize)]
struct Reference {
    id: usize,
    rule: usize,
}

#[derive(Serialize)]
struct Coherence {
    frame: usize,
    particle: Vec<Occurrence>,
}

#[derive(Serialize)]
struct Scope {
    parent: Option<usize>,
    particle: Vec<Reference>,
    held: Vec<Occurrence>,
}

#[derive(Serialize)]
pub struct Configuration {
    id: usize,
    world: Vec<Coherence>,
    frame: Vec<Scope>,
}

impl From<Token> for Occurrence {
    fn from(token: Token) -> Self {
        match token.value {
            Value::Atom(label) => Self::Atom {
                id: token.id,
                label,
            },
            Value::Rule(rule) => Self::Rule {
                id: token.id,
                rule,
                capture: token.capture,
            },
        }
    }
}

fn list(particle: Vec<Token>) -> Vec<Occurrence> {
    particle.into_iter().map(Occurrence::from).collect()
}

impl From<Frame> for Scope {
    fn from(frame: Frame) -> Self {
        Self {
            parent: frame.parent,
            particle: frame
                .particle
                .iter()
                .filter_map(|token| match token.value {
                    Value::Rule(rule) => Some(Reference { id: token.id, rule }),
                    Value::Atom(_) => None,
                })
                .collect(),
            held: list(frame.held),
        }
    }
}

impl From<Node> for Configuration {
    fn from(node: Node) -> Self {
        Self {
            id: node.id,
            world: node
                .world
                .into_iter()
                .map(|world| Coherence {
                    frame: world.frame,
                    particle: list(world.particle),
                })
                .collect(),
            frame: node.frame.into_iter().map(Scope::from).collect(),
        }
    }
}
