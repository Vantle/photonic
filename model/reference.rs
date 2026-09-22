use crate::application::{Code, Request};
use crate::comparison;
use crate::shape::Shape;
use std::collections::BTreeSet;

pub(crate) fn code(
    before: &crate::configuration::Configuration,
    after: &crate::configuration::Configuration,
    source: Code,
    target: Code,
    mapping: &comparison::Mapping,
    witness: &comparison::Mapping,
) -> bool {
    match (source, target) {
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
            let source = &before.frame[&left].declaration[original];
            let identity = after
                .frame()
                .map(|frame| (frame.identity, frame.identity))
                .collect();
            let target = &after.frame[&right].declaration[destination];
            Shape::declaration(std::slice::from_ref(source), mapping.frame())
                == Shape::declaration(std::slice::from_ref(target), &identity)
        }
        _ => false,
    }
}

pub(crate) fn selection(source: &Request, target: &Request, witness: &comparison::Mapping) -> bool {
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
