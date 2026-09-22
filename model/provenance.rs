use crate::comparison::Mapping;
use crate::flow::{Flow, Place};
use std::collections::{BTreeMap, BTreeSet};

pub fn place(mapping: &Mapping, source: Place) -> Option<Place> {
    let occurrence = *mapping.occurrence().get(&source.occurrence())?;
    Some(match source {
        Place::World(world, _) => Place::World(*mapping.world().get(&world)?, occurrence),
        Place::Held(frame, _) => Place::Held(*mapping.frame().get(&frame)?, occurrence),
    })
}

pub fn resource(mapping: &Mapping, source: &BTreeSet<Place>) -> Option<BTreeSet<Place>> {
    source
        .iter()
        .map(|&source| place(mapping, source))
        .collect()
}

pub fn origin(
    source: &Mapping,
    target: &Mapping,
    original: &crate::archive::Origin,
    destination: &crate::archive::Origin,
) -> bool {
    original.address == destination.address
        && source.frame().get(&original.context) == Some(&destination.context)
        && original.resource.len() == destination.resource.len()
        && original.resource.iter().all(|(identity, copy)| {
            source
                .occurrence()
                .get(identity)
                .and_then(|identity| destination.resource.get(identity))
                .is_some_and(|destination| target.occurrence().get(copy) == Some(destination))
        })
}

pub fn compare(source: &Mapping, target: &Mapping, original: &Flow, destination: &Flow) -> bool {
    if original.resource.len() != destination.resource.len()
        || original.context.len() != destination.context.len()
        || original.frame.len() != destination.frame.len()
    {
        return false;
    }
    for (&output, basis) in &original.resource {
        let Some(output) = place(target, output) else {
            return false;
        };
        let Some(basis) = resource(source, basis) else {
            return false;
        };
        if destination.resource.get(&output) != Some(&basis) {
            return false;
        }
    }
    for (output, basis) in &original.context {
        let Some(output) = target.world().get(output) else {
            return false;
        };
        let Some(basis) = basis
            .iter()
            .map(|world| source.world().get(world).copied())
            .collect::<Option<BTreeSet<_>>>()
        else {
            return false;
        };
        if destination.context.get(output) != Some(&basis) {
            return false;
        }
    }
    for (output, basis) in &original.frame {
        let Some(output) = target.frame().get(output) else {
            return false;
        };
        let basis = match basis {
            Some(frame) => {
                let Some(frame) = source.frame().get(frame) else {
                    return false;
                };
                Some(*frame)
            }
            None => None,
        };
        if destination.frame.get(output) != Some(&basis) {
            return false;
        }
    }
    true
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

pub(crate) fn persistent(source: &Mapping, target: &Mapping) -> bool {
    consistent(source.frame(), target.frame())
        && consistent(source.world(), target.world())
        && consistent(source.occurrence(), target.occurrence())
}
