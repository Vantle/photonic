use crate::application::Code;
use crate::capture::Capture;
use crate::failure::Failure;
use crate::flow::Place;
use crate::fragment::Fragment;
use crate::path::Step;
use crate::qualification::Qualification;
use crate::structure::Value;
use crate::support::{self, Address};
use crate::transition::Transition;
use std::collections::BTreeSet;

pub fn apply(
    source: &Transition,
    value: Fragment<Value<Capture, Capture>>,
) -> Result<Transition, Failure> {
    let Code::Local { world, occurrence } = source.request().code else {
        return Err(Failure::Rule);
    };
    let qualification = Qualification::new(source.path().clone(), world)?;
    let expected = qualification.inspect(support::Request {
        address: Address {
            derivation: vec![],
            state: source.path().record().len(),
        },
        world,
        place: Place::World(world, occurrence),
    })?;
    if value.evidence() != expected.evidence() {
        return Err(Failure::Witness(occurrence));
    }
    if value.value() != expected.value() {
        return Err(Failure::Match(world));
    }
    let path = source
        .path()
        .advance(Step::Construction(crate::publication::Request {
            world,
            consumed: vec![occurrence],
            value,
        }))?;
    let record = path.record().last().unwrap();
    let destination = *record
        .flow
        .context
        .iter()
        .find(|(_, basis)| **basis == BTreeSet::from([world]))
        .unwrap()
        .0;
    let emitted = record
        .flow
        .resource
        .iter()
        .find_map(|(place, basis)| match place {
            Place::World(owner, identity)
                if *owner == destination
                    && *basis == BTreeSet::from([Place::World(world, occurrence)]) =>
            {
                Some(*identity)
            }
            _ => None,
        })
        .unwrap();
    let original = source
        .path()
        .target()
        .occurrence(Place::World(world, occurrence));
    let replacement = path.target().occurrence(Place::World(destination, emitted));
    if original.value != replacement.value {
        return Err(Failure::Identity(occurrence));
    }
    let mut request = source.request().clone();
    request.code = Code::Local {
        world: destination,
        occurrence: emitted,
    };
    for selection in &mut request.selection {
        if selection.world != world {
            continue;
        }
        selection.world = destination;
        for identity in &mut selection.occurrence {
            if *identity == occurrence {
                *identity = emitted;
            }
        }
    }
    Transition::new(path, request)
}
