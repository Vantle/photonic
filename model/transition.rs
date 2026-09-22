use crate::application::{self, Code, Request};
use crate::comparison;
use crate::failure::Failure;
use crate::path::Path;
use crate::projection::{self, Binding};
use crate::provenance;
use crate::shape::Shape;
use std::collections::{BTreeMap, BTreeSet};

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

fn consistent<Identity: Ord>(
    source: &BTreeMap<Identity, Identity>,
    target: &BTreeMap<Identity, Identity>,
) -> bool {
    source.iter().all(|(left, right)| {
        target.get(left).is_none_or(|value| value == right)
            && target
                .iter()
                .all(|(other, value)| value != right || other == left)
    })
}

fn persistent(source: &comparison::Mapping, target: &comparison::Mapping) -> bool {
    consistent(source.frame(), target.frame())
        && consistent(source.world(), target.world())
        && consistent(source.occurrence(), target.occurrence())
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

fn binding(mapping: &comparison::Mapping, source: &Binding, target: &Binding) -> bool {
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

fn code(
    source: &Transition,
    target: &Transition,
    mapping: &comparison::Mapping,
    witness: &comparison::Mapping,
) -> bool {
    match (source.request.code, target.request.code) {
        (
            Code::Local {
                world: left,
                occurrence: original,
            },
            Code::Local {
                world: right,
                occurrence: destination,
            },
        ) => {
            witness.world().get(&left) == Some(&right)
                && witness.occurrence().get(&original) == Some(&destination)
        }
        (
            Code::Declaration {
                context: left,
                position: original,
            },
            Code::Declaration {
                context: right,
                position: destination,
            },
        ) => {
            if mapping.frame().get(&left) != Some(&right) {
                return false;
            }
            let source = &source.path.source().frame[&left].declaration[original];
            let identity = target
                .path
                .source()
                .frame()
                .map(|frame| (frame.identity, frame.identity))
                .collect();
            let target = &target.path.source().frame[&right].declaration[destination];
            Shape::declaration(std::slice::from_ref(source), mapping.frame())
                == Shape::declaration(std::slice::from_ref(target), &identity)
        }
        _ => false,
    }
}

fn selection(source: &Request, target: &Request, witness: &comparison::Mapping) -> bool {
    let mut mapped = Vec::new();
    for selection in &source.selection {
        let Some(&world) = witness.world().get(&selection.world) else {
            return false;
        };
        let Some(occurrence) = selection
            .occurrence
            .iter()
            .map(|identity| witness.occurrence().get(identity).copied())
            .collect::<Option<BTreeSet<_>>>()
        else {
            return false;
        };
        mapped.push((world, occurrence));
    }
    mapped.sort();
    let mut expected = target
        .selection
        .iter()
        .map(|selection| {
            (
                selection.world,
                selection
                    .occurrence
                    .iter()
                    .copied()
                    .collect::<BTreeSet<_>>(),
            )
        })
        .collect::<Vec<_>>();
    expected.sort();
    mapped == expected
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
            if !persistent(mapping, witness)
                || !provenance::compare(mapping, witness, source.path.flow(), target.path.flow())
                || !selection(&source.request, &target.request, witness)
                || !code(source, target, mapping, witness)
            {
                return false;
            }
            comparison::find(&source.event.target, &target.event.target, |destination| {
                if !persistent(mapping, destination)
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
