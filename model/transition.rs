use crate::application::{self, Request};
use crate::comparison;
use crate::failure::Failure;
use crate::path::Path;
use crate::projection::{self, Binding};
use crate::provenance;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transition {
    path: Path,
    request: Request,
    binding: Binding,
    event: application::Event,
}

impl Transition {
    pub fn new(path: Path, request: Request) -> Result<Self, Failure> {
        let projection = projection::project(&path, request.clone())?;
        let event = projection.apply()?;
        Ok(Self {
            binding: projection.binding().clone(),
            path,
            request,
            event,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn request(&self) -> &Request {
        &self.request
    }
    pub fn binding(&self) -> &Binding {
        &self.binding
    }
    pub fn event(&self) -> &application::Event {
        &self.event
    }

    pub fn retain(&self) -> Result<Path, Failure> {
        Path::new(self.path.source().clone()).advance(crate::path::Step::Inference {
            path: Box::new(self.path.clone()),
            request: self.request.clone(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Mapping {
    source: comparison::Mapping,
    witness: comparison::Mapping,
    target: comparison::Mapping,
}

impl Mapping {
    pub fn source(&self) -> &comparison::Mapping {
        &self.source
    }
    pub fn witness(&self) -> &comparison::Mapping {
        &self.witness
    }
    pub fn target(&self) -> &comparison::Mapping {
        &self.target
    }
}

fn owner(
    mapping: &comparison::Mapping,
    source: crate::context::Identity,
    target: crate::context::Identity,
) -> bool {
    match mapping.frame().get(&source) {
        Some(&identity) => identity == target,
        None => !mapping.frame().values().any(|&identity| identity == target),
    }
}

pub(crate) fn binding(mapping: &comparison::Mapping, source: &Binding, target: &Binding) -> bool {
    mapping.frame().get(&source.frame) == Some(&target.frame)
        && match source.owner {
            Some(owner) => mapping.frame().get(&owner).copied() == target.owner,
            None => target.owner.is_none(),
        }
        && source
            .world
            .iter()
            .map(|world| mapping.world().get(world).copied())
            .collect::<Option<BTreeSet<_>>>()
            .as_ref()
            == Some(&target.world)
        && provenance::resource(mapping, &source.footprint).as_ref() == Some(&target.footprint)
        && provenance::resource(mapping, &source.exact).as_ref() == Some(&target.exact)
        && provenance::resource(mapping, &source.read).as_ref() == Some(&target.read)
}

pub fn boundary(source: &Transition, target: &Transition) -> Option<Mapping> {
    find(source, target, |_| true)
}

pub fn find(
    source: &Transition,
    target: &Transition,
    mut accept: impl FnMut(&Mapping) -> bool,
) -> Option<Mapping> {
    let mut result = None;
    comparison::find(source.path.source(), target.path.source(), |mapping| {
        if !binding(mapping, &source.binding, &target.binding) {
            return false;
        }
        comparison::find(source.path.target(), target.path.target(), |witness| {
            if !provenance::persistent(mapping, witness)
                || !provenance::compare(mapping, witness, source.path.flow(), target.path.flow())
                || !crate::reference::selection(&source.request, &target.request, witness)
                || !crate::reference::code(
                    source.path.source(),
                    target.path.source(),
                    source.request.code,
                    target.request.code,
                    mapping,
                    witness,
                )
            {
                return false;
            }
            comparison::find(&source.event.target, &target.event.target, |destination| {
                if !provenance::persistent(mapping, destination)
                    || !owner(mapping, source.event.owner, target.event.owner)
                    || !owner(destination, source.event.owner, target.event.owner)
                    || !provenance::compare(
                        mapping,
                        destination,
                        &source.event.flow,
                        &target.event.flow,
                    )
                    || provenance::resource(mapping, &source.event.read).as_ref()
                        != Some(&target.event.read)
                    || provenance::resource(mapping, &source.event.consumed).as_ref()
                        != Some(&target.event.consumed)
                {
                    return false;
                }
                let candidate = Mapping {
                    source: mapping.clone(),
                    witness: witness.clone(),
                    target: destination.clone(),
                };
                if !accept(&candidate) {
                    return false;
                }
                result = Some(candidate);
                true
            })
            .is_some()
        })
        .is_some()
    });
    result
}
