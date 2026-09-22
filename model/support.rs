use crate::configuration::Configuration;
use crate::failure::Failure;
use crate::flow::{Flow, Place};
use crate::occurrence::Occurrence;
use crate::path::{Path, Step};
use crate::world;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Address {
    pub derivation: Vec<usize>,
    pub state: usize,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Request {
    pub address: Address,
    pub world: world::Identity,
    pub place: Place,
}

#[derive(Debug)]
pub struct Support<'a> {
    path: &'a Path,
    world: world::Identity,
    request: &'a Request,
    state: &'a Configuration,
    occurrence: &'a Occurrence,
    flow: Flow,
    read: BTreeSet<Place>,
}

impl Support<'_> {
    pub fn path(&self) -> &Path {
        self.path
    }

    pub fn world(&self) -> world::Identity {
        self.world
    }

    pub fn request(&self) -> &Request {
        self.request
    }

    pub fn state(&self) -> &Configuration {
        self.state
    }

    pub fn occurrence(&self) -> &Occurrence {
        self.occurrence
    }

    pub fn flow(&self) -> &Flow {
        &self.flow
    }

    pub fn read(&self) -> &BTreeSet<Place> {
        &self.read
    }
}

fn project(
    flow: &Flow,
    world: &BTreeSet<world::Identity>,
) -> Result<BTreeSet<world::Identity>, Failure> {
    let mut result = BTreeSet::new();
    for identity in world {
        result.extend(
            flow.context
                .get(identity)
                .ok_or(Failure::World(*identity))?,
        );
    }
    Ok(result)
}

fn occurrence<'a>(state: &'a Configuration, request: &Request) -> Result<&'a Occurrence, Failure> {
    let world = state
        .world
        .get(&request.world)
        .ok_or(Failure::World(request.world))?;
    let available = match request.place {
        Place::World(identity, _) => {
            if identity != request.world {
                return Err(Failure::World(identity));
            }
            &world.occurrence
        }
        Place::Held(context, _) => {
            if context != world.context {
                return Err(Failure::Owner {
                    world: request.world,
                    context,
                });
            }
            &state.frame[&context].held
        }
    };
    let identity = request.place.occurrence();
    available
        .iter()
        .find(|value| value.identity == identity)
        .ok_or(Failure::Occurrence(identity))
}

fn child(path: &Path, depth: usize, position: usize) -> Result<&Path, Failure> {
    let Some(record) = path.record().get(position) else {
        return Err(Failure::Derivation { depth, position });
    };
    let Step::Inference { path: child, .. } = &record.step else {
        return Err(Failure::Derivation { depth, position });
    };
    if child.source() != &path.state()[position] {
        return Err(Failure::Source);
    }
    Ok(child)
}

pub(crate) fn descend<'a>(path: &'a Path, derivation: &[usize]) -> Result<&'a Path, Failure> {
    let mut cursor = path;
    for (depth, &position) in derivation.iter().enumerate() {
        cursor = child(cursor, depth, position)?;
    }
    Ok(cursor)
}

pub fn resolve<'a>(
    path: &'a Path,
    world: world::Identity,
    request: &'a Request,
) -> Result<Support<'a>, Failure> {
    if !path.target().world.contains_key(&world) {
        return Err(Failure::World(world));
    }
    let mut cursor = path;
    let mut parent = Vec::new();
    let mut flow = Flow::identity(path.source());
    for (depth, &position) in request.address.derivation.iter().enumerate() {
        let child = child(cursor, depth, position)?;
        flow = flow
            .compose(cursor.prefix(position)?)
            .map_err(Failure::Flow)?;
        parent.push((cursor, position));
        cursor = child;
    }
    let state = cursor
        .state()
        .get(request.address.state)
        .ok_or(Failure::State(request.address.state))?;
    let occurrence = occurrence(state, request)?;
    path.target().history.permits(&state.history)?;
    flow = flow
        .compose(cursor.prefix(request.address.state)?)
        .map_err(Failure::Flow)?;
    flow.validate(path.source(), state).map_err(Failure::Flow)?;
    let mut position = request.address.state;
    let mut basis = BTreeSet::from([request.world]);
    for (source, anchor) in parent.into_iter().rev() {
        basis = project(cursor.prefix(position)?, &basis)?;
        cursor = source;
        position = anchor;
    }
    let mut lineage = BTreeSet::from([world]);
    for record in path.record()[position..].iter().rev() {
        lineage = project(&record.flow, &lineage)?;
    }
    if let Some(&source) = basis.difference(&lineage).next() {
        return Err(Failure::Lineage {
            source,
            target: world,
        });
    }
    let read =
        if request.address.derivation.is_empty() && request.address.state == path.record().len() {
            BTreeSet::from([request.place])
        } else {
            BTreeSet::new()
        };
    Ok(Support {
        path,
        world,
        request,
        state,
        occurrence,
        flow,
        read,
    })
}
