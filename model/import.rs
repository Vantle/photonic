use crate::activation;
use crate::allocation::take;
use crate::configuration::Configuration;
use crate::context::{self, Frame};
use crate::failure::Failure;
use crate::flow::{Flow, Place};
use crate::occurrence::{self, Occurrence};
use crate::path::Path;
use crate::structure::{Rule, Value};
use std::collections::{BTreeMap, BTreeSet};

fn rename(
    rule: Rule,
    frame: &BTreeMap<context::Identity, context::Identity>,
) -> Result<Rule, Failure> {
    let Value::Rule(rule) =
        activation::rename(
            Value::Rule(Box::new(rule)),
            &|identity| Ok(frame[&identity]),
        )?
    else {
        unreachable!()
    };
    Ok(*rule)
}

pub(crate) fn include(
    path: &Path,
    rule: &Rule,
    target: &mut Configuration,
    flow: &mut Flow,
) -> Result<(Rule, BTreeMap<context::Identity, crate::archive::Origin>), Failure> {
    let mut pending = BTreeSet::new();
    rule.collect(&mut pending);
    let mut imported = BTreeSet::new();
    let mut frame = path
        .flow()
        .frame
        .iter()
        .filter_map(|(&identity, &origin)| origin.map(|origin| (identity, origin)))
        .collect::<BTreeMap<_, _>>();
    while let Some(identity) = pending.pop_first() {
        if frame.contains_key(&identity) || !imported.insert(identity) {
            continue;
        }
        let original = &path.target().frame[&identity];
        pending.extend(original.parent);
        pending.extend(original.lexical);
        for rule in &original.declaration {
            rule.collect(&mut pending);
        }
        for value in &original.held {
            value.value.context(&mut pending);
        }
    }
    for &identity in &imported {
        frame.insert(
            identity,
            context::Identity(take(&mut target.allocation.context)?),
        );
    }
    let mut resource = BTreeMap::new();
    for &identity in &imported {
        for value in &path.target().frame[&identity].held {
            let basis = &path.flow().resource[&Place::Held(identity, value.identity)];
            if basis.len() != 1 {
                continue;
            }
            let original = path.source().occurrence(*basis.first().unwrap());
            let mapped = activation::rename(value.value.clone(), &|identity| {
                path.flow().frame[&identity].ok_or(Failure::Origin(identity))
            });
            if mapped.is_ok_and(|value| value == original.value)
                && value.history == original.history
            {
                resource.insert(value.identity, original.identity);
            }
        }
    }
    let address = crate::support::Address {
        derivation: vec![],
        state: path.record().len(),
    };
    let registry = crate::resource::Registry::new(path)?;
    let mut archive = BTreeMap::new();
    for identity in imported {
        let original = &path.target().frame[&identity];
        let destination = frame[&identity];
        let capture = crate::capture::resolve(path, &address, identity)?;
        let cursor = crate::support::descend(path, &capture.address().derivation)?;
        let canonical = &cursor.state()[capture.address().state].frame[&capture.identity()];
        let mut origin = crate::archive::Origin {
            address: capture.address().clone(),
            context: capture.identity(),
            resource: BTreeMap::new(),
        };
        let mut held = BTreeMap::new();
        for value in &original.held {
            let resource = match resource.entry(value.identity) {
                std::collections::btree_map::Entry::Occupied(entry) => *entry.get(),
                std::collections::btree_map::Entry::Vacant(entry) => *entry.insert(
                    occurrence::Identity(take(&mut target.allocation.occurrence)?),
                ),
            };
            let key = registry.resolve(path, &address, value.identity)?;
            for source in &canonical.held {
                if registry.resolve(path, capture.address(), source.identity)? == key {
                    origin.resource.insert(source.identity, resource);
                }
            }
            let occurrence = Occurrence {
                identity: resource,
                value: activation::rename(value.value.clone(), &|identity| Ok(frame[&identity]))?,
                history: value.history.clone(),
            };
            if held
                .insert(resource, occurrence.clone())
                .is_some_and(|previous| previous != occurrence)
            {
                return Err(Failure::Identity(resource));
            }
            flow.resource
                .entry(Place::Held(destination, resource))
                .or_default()
                .extend(&path.flow().resource[&Place::Held(identity, value.identity)]);
        }
        target.frame.insert(
            destination,
            Frame {
                identity: destination,
                parent: original.parent.map(|identity| frame[&identity]),
                lexical: original.lexical.map(|identity| frame[&identity]),
                declaration: original
                    .declaration
                    .iter()
                    .cloned()
                    .map(|rule| rename(rule, &frame))
                    .collect::<Result<_, _>>()?,
                held: held.into_values().collect(),
            },
        );
        flow.frame.insert(destination, None);
        archive.insert(destination, origin);
    }
    Ok((rename(rule.clone(), &frame)?, archive))
}
