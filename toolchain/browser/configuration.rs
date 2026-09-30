use photonic::snapshot::{Frame, Node, Token, Value};
use serde::Serialize;
use spectrum::order::Naming;
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

fn occurrence(token: Token, naming: &Naming) -> Occurrence {
    match token.value {
        Value::Atom(label) => Occurrence::Atom {
            id: token.id,
            label: Arc::from(naming.name(&label)),
        },
        Value::Rule(rule) => Occurrence::Rule {
            id: token.id,
            rule,
            capture: token.capture,
        },
    }
}

fn list(particle: Vec<Token>, naming: &Naming) -> Vec<Occurrence> {
    particle
        .into_iter()
        .map(|token| occurrence(token, naming))
        .collect()
}

fn scope(frame: Frame, naming: &Naming) -> Scope {
    Scope {
        parent: frame.parent,
        particle: frame
            .particle
            .iter()
            .filter_map(|token| match token.value {
                Value::Rule(rule) => Some(Reference { id: token.id, rule }),
                Value::Atom(_) => None,
            })
            .collect(),
        held: list(frame.held, naming),
    }
}

impl Configuration {
    // A configuration of the canonical program's run, under the handle the numbering gives it and in
    // the source's names.
    pub fn new(node: Node, id: usize, naming: &Naming) -> Self {
        Self {
            id,
            world: node
                .world
                .into_iter()
                .map(|world| Coherence {
                    frame: world.frame,
                    particle: list(world.particle, naming),
                })
                .collect(),
            frame: node
                .frame
                .into_iter()
                .map(|frame| scope(frame, naming))
                .collect(),
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }
}

impl From<Node> for Configuration {
    fn from(node: Node) -> Self {
        let id = node.id;
        Self::new(node, id, &Naming::default())
    }
}
