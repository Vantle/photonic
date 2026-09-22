use crate::comparison::Mapping;
use crate::derivation::Derivation;
use crate::evidence::Evidence;
use crate::fragment::Fragment;
use crate::path::{Path, Step};
use crate::provenance;
use crate::shape::Shape;
use crate::structure::Value;
use std::collections::{BTreeMap, BTreeSet};

fn set<Identity: Copy + Ord>(
    mapping: &BTreeMap<Identity, Identity>,
    source: &BTreeSet<Identity>,
    target: &BTreeSet<Identity>,
) -> bool {
    source
        .iter()
        .map(|identity| mapping.get(identity).copied())
        .collect::<Option<BTreeSet<_>>>()
        .as_ref()
        == Some(target)
}

fn occurrence(
    source: &crate::occurrence::Occurrence,
    target: &crate::occurrence::Occurrence,
    mapping: &Mapping,
) -> bool {
    if mapping.occurrence().get(&source.identity) != Some(&target.identity)
        || source.history != target.history
    {
        return false;
    }
    let identity = mapping
        .frame()
        .values()
        .map(|&identity| (identity, identity))
        .collect();
    Shape::value(&source.value, mapping.frame()) == Shape::value(&target.value, &identity)
}

fn evidence(source: &Evidence, target: &Evidence, mapping: &Mapping) -> bool {
    if source.history != target.history
        || source.read.len() != target.read.len()
        || !set(mapping.frame(), &source.context, &target.context)
    {
        return false;
    }
    source.read.iter().all(|(identity, value)| {
        mapping
            .occurrence()
            .get(identity)
            .and_then(|identity| target.read.get(identity))
            .is_some_and(|target| occurrence(value, target, mapping))
    })
}

fn nominal(source: &Fragment<Value>, target: &Fragment<Value>, mapping: &Mapping) -> bool {
    let left = source.evidence();
    let right = target.evidence();
    if left.proof.is_some()
        || right.proof.is_some()
        || !left.qualified.is_empty()
        || !right.qualified.is_empty()
        || !left.capture.is_empty()
        || !right.capture.is_empty()
        || !evidence(left, right, mapping)
    {
        return false;
    }
    let identity = mapping
        .frame()
        .values()
        .map(|&identity| (identity, identity))
        .collect();
    Shape::value(source.value(), mapping.frame()) == Shape::value(target.value(), &identity)
}

fn qualified(
    left: &Path,
    right: &Path,
    source: &Fragment<Value<crate::capture::Capture, crate::capture::Capture>>,
    target: &Fragment<Value<crate::capture::Capture, crate::capture::Capture>>,
    derivation: &Derivation,
    mapping: &Mapping,
) -> bool {
    let original = source.evidence();
    let destination = target.evidence();
    if !evidence(original, destination, mapping)
        || original.qualified.len() != destination.qualified.len()
        || original.capture.len() != destination.capture.len()
    {
        return false;
    }
    let (Some(source), Some(target)) = (original.proof(), destination.proof()) else {
        return false;
    };
    let position = source.path().record().len();
    if position != target.path().record().len()
        || !left.state().starts_with(source.path().state())
        || !left.record().starts_with(source.path().record())
        || !right.state().starts_with(target.path().state())
        || !right.record().starts_with(target.path().record())
        || derivation.state[position].world().get(&source.world()) != Some(&target.world())
    {
        return false;
    }
    for (request, value) in &original.qualified {
        let Some(mapping) = derivation.at(&request.address) else {
            return false;
        };
        let Some(&world) = mapping.world().get(&request.world) else {
            return false;
        };
        let Some(place) = provenance::place(mapping, request.place) else {
            return false;
        };
        let request = crate::support::Request {
            address: request.address.clone(),
            world,
            place,
        };
        let Some(target) = destination.qualified.get(&request) else {
            return false;
        };
        if !occurrence(value, target, mapping) {
            return false;
        }
    }
    true
}

fn construction(
    left: &Path,
    right: &Path,
    source: &crate::publication::Request,
    target: &crate::publication::Request,
    derivation: &Derivation,
    mapping: &Mapping,
) -> bool {
    if !qualified(
        left,
        right,
        &source.value,
        &target.value,
        derivation,
        mapping,
    ) {
        return false;
    }
    let original = source.value.evidence();
    let destination = target.value.evidence();
    let mut capture = BTreeMap::new();
    for value in &original.capture {
        let Some(mapping) = derivation.at(value.address()) else {
            return false;
        };
        let Some(&identity) = mapping.frame().get(&value.identity()) else {
            return false;
        };
        let Some(target) = destination
            .capture
            .iter()
            .find(|target| target.address() == value.address() && target.identity() == identity)
        else {
            return false;
        };
        capture.insert(value.clone(), target.clone());
    }
    let identity = destination
        .capture
        .iter()
        .map(|capture| (capture.clone(), capture.clone()))
        .collect();
    Shape::value(source.value.value(), &capture) == Shape::value(target.value.value(), &identity)
}

fn consumed(
    source: &[crate::occurrence::Identity],
    target: &[crate::occurrence::Identity],
    mapping: &Mapping,
) -> bool {
    set(
        mapping.occurrence(),
        &source.iter().copied().collect(),
        &target.iter().copied().collect(),
    )
}

fn witness(
    source: &[crate::admission::Witness],
    target: &[crate::admission::Witness],
    derivation: &Derivation,
) -> bool {
    if source.len() != target.len() {
        return false;
    }
    let mut result = BTreeSet::new();
    for source in source {
        let Some(mapping) = derivation.state.get(source.state) else {
            return false;
        };
        let Some(&world) = mapping.world().get(&source.world) else {
            return false;
        };
        let Some(place) = provenance::place(mapping, source.place) else {
            return false;
        };
        result.insert(crate::admission::Witness {
            state: source.state,
            world,
            place,
        });
    }
    result == target.iter().copied().collect()
}

pub(crate) fn check(left: &Path, right: &Path, derivation: &Derivation) -> bool {
    let union = derivation.identity();
    for (position, (source, target)) in left.record().iter().zip(right.record()).enumerate() {
        let mapping = &derivation.state[position];
        if provenance::resource(mapping, &source.read).as_ref() != Some(&target.read)
            || provenance::resource(mapping, &source.consumed).as_ref() != Some(&target.consumed)
            || !set(union.frame(), &source.context, &target.context)
        {
            return false;
        }
        let matched = match (&source.step, &target.step) {
            (Step::Application(source), Step::Application(target)) => {
                crate::reference::code(
                    &left.state()[position],
                    &right.state()[position],
                    source.code,
                    target.code,
                    mapping,
                    mapping,
                ) && crate::reference::selection(source, target, mapping)
            }
            (
                Step::Inference {
                    path: source,
                    request: original,
                },
                Step::Inference {
                    path: target,
                    request: destination,
                },
            ) => {
                let child = &derivation.branch[&position];
                let witness = child.state.last().unwrap();
                let original = crate::projection::project(source, original.clone()).unwrap();
                let destination = crate::projection::project(target, destination.clone()).unwrap();
                crate::reference::code(
                    source.source(),
                    target.source(),
                    original.request().code,
                    destination.request().code,
                    mapping,
                    witness,
                ) && crate::reference::selection(original.request(), destination.request(), witness)
                    && crate::transition::binding(
                        mapping,
                        original.binding(),
                        destination.binding(),
                    )
            }
            (
                Step::Introduction {
                    world: source,
                    consumed: original,
                    value,
                },
                Step::Introduction {
                    world: target,
                    consumed: destination,
                    value: other,
                },
            ) => {
                mapping.world().get(source) == Some(target)
                    && consumed(original, destination, mapping)
                    && nominal(value, other, union)
            }
            (Step::Historical(source), Step::Historical(target)) => {
                mapping.world().get(&source.world) == Some(&target.world)
                    && consumed(&source.consumed, &target.consumed, mapping)
                    && witness(&source.witness, &target.witness, derivation)
                    && nominal(&source.value, &target.value, union)
            }
            (Step::Construction(source), Step::Construction(target)) => {
                mapping.world().get(&source.world) == Some(&target.world)
                    && consumed(&source.consumed, &target.consumed, mapping)
                    && construction(left, right, source, target, derivation, union)
            }
            _ => false,
        };
        if !matched {
            return false;
        }
    }
    true
}
