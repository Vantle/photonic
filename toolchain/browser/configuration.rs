use crate::catalog::Catalog;
use crate::failure::Failure;
use photonic::snapshot::{Frame, Kind, Node, Token, World};
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

impl Occurrence {
    fn new(token: Token, catalog: &Catalog) -> Result<Self, Failure> {
        match token.kind {
            Kind::Atom => Ok(Self::Atom {
                id: token.id,
                label: token.label,
            }),
            Kind::Rule => Ok(Self::Rule {
                id: token.id,
                rule: catalog.rule(&token.label)?,
                capture: token.capture,
            }),
        }
    }

    fn list(particle: Vec<Token>, catalog: &Catalog) -> Result<Vec<Self>, Failure> {
        particle
            .into_iter()
            .map(|token| Self::new(token, catalog))
            .collect()
    }
}

impl Reference {
    fn new(token: &Token, catalog: &Catalog) -> Result<Self, Failure> {
        Ok(Self {
            id: token.id,
            rule: catalog.rule(&token.label)?,
        })
    }
}

impl Coherence {
    fn new(world: World, catalog: &Catalog) -> Result<Self, Failure> {
        Ok(Self {
            frame: world.frame,
            particle: Occurrence::list(world.particle, catalog)?,
        })
    }
}

impl Scope {
    fn new(frame: Frame, catalog: &Catalog) -> Result<Self, Failure> {
        Ok(Self {
            parent: frame.parent,
            particle: frame
                .particle
                .iter()
                .map(|token| Reference::new(token, catalog))
                .collect::<Result<_, _>>()?,
            held: Occurrence::list(frame.held, catalog)?,
        })
    }
}

impl Configuration {
    pub fn new(node: Node, catalog: &Catalog) -> Result<Self, Failure> {
        Ok(Self {
            id: node.id,
            world: node
                .world
                .into_iter()
                .map(|world| Coherence::new(world, catalog))
                .collect::<Result<_, _>>()?,
            frame: node
                .frame
                .into_iter()
                .map(|frame| Scope::new(frame, catalog))
                .collect::<Result<_, _>>()?,
        })
    }
}
