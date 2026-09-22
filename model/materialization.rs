use crate::activation;
use crate::allocation::take;
use crate::archive::Origin;
use crate::capture::{self, Capture};
use crate::configuration::Configuration;
use crate::context::{self, Frame};
use crate::failure::Failure;
use crate::flow::{Flow, Place};
use crate::occurrence::{self, Occurrence};
use crate::path::Path;
use crate::resource;
use crate::structure::Value;
use crate::support::{self, Address};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct Materialization {
    pub frame: BTreeMap<Capture, context::Identity>,
    pub origin: BTreeMap<context::Identity, Origin>,
}

fn original<'a>(path: &'a Path, capture: &Capture) -> Result<&'a Frame, Failure> {
    let cursor = support::descend(path, &capture.address().derivation)?;
    let state = cursor
        .state()
        .get(capture.address().state)
        .ok_or(Failure::State(capture.address().state))?;
    state
        .frame
        .get(&capture.identity())
        .ok_or(Failure::Context(capture.identity()))
}

pub(crate) fn include(
    path: &Path,
    capture: &BTreeSet<Capture>,
    target: &mut Configuration,
    flow: &mut Flow,
) -> Result<Materialization, Failure> {
    let current = Address {
        derivation: vec![],
        state: path.record().len(),
    };
    let mut frame = BTreeMap::new();
    for source in path.target().frame() {
        frame
            .entry(capture::resolve(path, &current, source.identity)?)
            .or_insert(source.identity);
    }
    let mut pending = capture.clone();
    let mut selected = BTreeMap::new();
    while let Some(capture) = pending.pop_first() {
        if frame.contains_key(&capture) || selected.contains_key(&capture) {
            continue;
        }
        let source = original(path, &capture)?;
        let mut context = BTreeSet::new();
        context.extend(source.parent);
        context.extend(source.lexical);
        for rule in &source.declaration {
            rule.collect(&mut context);
        }
        for value in &source.held {
            value.value.context(&mut context);
        }
        for identity in context {
            pending.insert(capture::resolve(path, capture.address(), identity)?);
        }
        selected.insert(capture, source);
    }
    for capture in selected.keys() {
        frame.insert(
            capture.clone(),
            context::Identity(take(&mut target.allocation.context)?),
        );
    }
    let registry = resource::Registry::new(path)?;
    let mut available = BTreeMap::new();
    for &place in flow.resource.keys() {
        let value = path.target().occurrence(place);
        let origin = registry.resolve(path, &current, value.identity)?;
        let qualified = activation::rename(value.value.clone(), &|identity| {
            capture::resolve(path, &current, identity)
        })?;
        let entry = available
            .entry(origin)
            .or_insert_with(|| (value, qualified, BTreeSet::new()));
        entry.2.insert(place);
    }
    let mut created = BTreeMap::<_, Occurrence>::new();
    let mut origin = BTreeMap::new();
    for (capture, source) in selected {
        let destination = frame[&capture];
        let context = |identity| Ok(frame[&capture::resolve(path, capture.address(), identity)?]);
        let mut held = BTreeMap::new();
        let mut resource = BTreeMap::new();
        for value in &source.held {
            target.history.permits(&value.history)?;
            let key = registry.resolve(path, capture.address(), value.identity)?;
            let qualified = activation::rename(value.value.clone(), &|identity| {
                capture::resolve(path, capture.address(), identity)
            })?;
            let (occurrence, basis) = if let Some((current, expected, basis)) = available.get(&key)
            {
                if qualified != *expected || value.history != current.history {
                    return Err(Failure::Identity(value.identity));
                }
                ((*current).clone(), basis.clone())
            } else {
                let mapped = activation::rename(value.value.clone(), &context)?;
                let occurrence = match created.entry(key) {
                    std::collections::btree_map::Entry::Occupied(entry) => {
                        if entry.get().value != mapped || entry.get().history != value.history {
                            return Err(Failure::Identity(value.identity));
                        }
                        entry.get().clone()
                    }
                    std::collections::btree_map::Entry::Vacant(entry) => entry
                        .insert(Occurrence {
                            identity: occurrence::Identity(take(
                                &mut target.allocation.occurrence,
                            )?),
                            value: mapped,
                            history: value.history.clone(),
                        })
                        .clone(),
                };
                (occurrence, BTreeSet::new())
            };
            resource.insert(value.identity, occurrence.identity);
            flow.resource
                .entry(Place::Held(destination, occurrence.identity))
                .or_default()
                .extend(basis);
            if held
                .insert(occurrence.identity, occurrence.clone())
                .is_some_and(|previous| previous != occurrence)
            {
                return Err(Failure::Identity(occurrence.identity));
            }
        }
        let declaration = source
            .declaration
            .iter()
            .cloned()
            .map(|rule| {
                let Value::Rule(rule) = activation::rename(Value::Rule(Box::new(rule)), &context)?
                else {
                    unreachable!()
                };
                Ok(*rule)
            })
            .collect::<Result<_, Failure>>()?;
        target.frame.insert(
            destination,
            Frame {
                identity: destination,
                parent: source.parent.map(context).transpose()?,
                lexical: source.lexical.map(context).transpose()?,
                declaration,
                held: held.into_values().collect(),
            },
        );
        flow.frame.insert(destination, None);
        origin.insert(
            destination,
            Origin {
                address: capture.address().clone(),
                context: capture.identity(),
                resource,
            },
        );
    }
    Ok(Materialization { frame, origin })
}
