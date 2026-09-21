use crate::activation;
use crate::allocation::take;
use crate::configuration::Configuration;
use crate::context::{self, Frame};
use crate::failure::Failure;
use crate::flow::{Flow, Place};
use crate::occurrence::{self, Occurrence};
use crate::path::Path;
use crate::structure::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Origin {
    pub state: usize,
    pub context: context::Identity,
    pub resource: BTreeMap<occurrence::Identity, occurrence::Identity>,
}

pub(crate) struct Archive {
    pub frame: BTreeMap<context::Identity, context::Identity>,
    pub origin: BTreeMap<context::Identity, Origin>,
}

fn component<Identity: Copy + Ord>(
    identity: Identity,
    origin: &BTreeMap<Identity, Vec<Identity>>,
) -> BTreeSet<Identity> {
    let mut pending = vec![identity];
    let mut visited = BTreeSet::new();
    while let Some(identity) = pending.pop() {
        if !visited.insert(identity) {
            continue;
        }
        pending.extend(origin.get(&identity).into_iter().flatten().copied());
    }
    visited
}

pub(crate) fn restore(
    path: &Path,
    context: &BTreeSet<context::Identity>,
    target: &mut Configuration,
    flow: &mut Flow,
) -> Result<Archive, Failure> {
    let source = path.target();
    let available = source.frame.keys().copied().collect();
    let mut ancestry = BTreeMap::<_, Vec<_>>::new();
    let mut inheritance = BTreeMap::<_, Vec<_>>::new();
    for record in path.record() {
        for (&identity, origin) in &record.archive {
            ancestry.entry(origin.context).or_default().push(identity);
            ancestry.entry(identity).or_default().push(origin.context);
            for (&source, &identity) in &origin.resource {
                inheritance.entry(source).or_default().push(identity);
                inheritance.entry(identity).or_default().push(source);
            }
        }
    }
    let mut frame = source
        .frame
        .keys()
        .map(|&identity| (identity, identity))
        .collect::<BTreeMap<_, _>>();
    let mut pending = context.clone();
    let mut selected = BTreeMap::new();
    let mut representative = BTreeMap::new();
    while let Some(identity) = pending.pop_first() {
        if frame.contains_key(&identity) || representative.contains_key(&identity) {
            continue;
        }
        let equivalent = component(identity, &ancestry);
        if let Some(&destination) = equivalent.intersection(&available).next() {
            frame.extend(
                equivalent
                    .into_iter()
                    .map(|identity| (identity, destination)),
            );
            continue;
        }
        let (identity, position, original) = equivalent
            .iter()
            .rev()
            .find_map(|&identity| {
                path.state()
                    .iter()
                    .enumerate()
                    .rev()
                    .find_map(|(position, state)| {
                        state
                            .frame
                            .get(&identity)
                            .map(|frame| (identity, position, frame))
                    })
            })
            .ok_or(Failure::Context(identity))?;
        representative.extend(equivalent.into_iter().map(|source| (source, identity)));
        selected.insert(identity, (position, original));
        pending.extend(original.parent);
        pending.extend(original.lexical);
        for rule in &original.declaration {
            rule.collect(&mut pending);
        }
        for value in &original.held {
            value.value.context(&mut pending);
        }
    }
    let mut origin = BTreeMap::new();
    for (&identity, &(state, _)) in &selected {
        let destination = context::Identity(take(&mut target.allocation.context)?);
        frame.insert(identity, destination);
        origin.insert(
            destination,
            Origin {
                state,
                context: identity,
                resource: BTreeMap::new(),
            },
        );
    }
    for (identity, source) in representative {
        frame.insert(identity, frame[&source]);
    }
    let mut available = BTreeMap::new();
    for &place in flow.resource.keys() {
        let value = source.occurrence(place);
        let entry = available
            .entry(value.identity)
            .or_insert_with(|| (value, BTreeSet::new()));
        entry.1.insert(place);
    }
    let retained = available.keys().copied().collect();
    let mut resource = BTreeMap::new();
    for (&identity, &(_, original)) in &selected {
        let destination = frame[&identity];
        let mut held = BTreeMap::new();
        for value in &original.held {
            source.history.permits(&value.history)?;
            let mapped = activation::rename(value.value.clone(), &|identity| Ok(frame[&identity]))?;
            let equivalent = component(value.identity, &inheritance);
            let (identity, basis) =
                if let Some(identity) = equivalent.intersection(&retained).next() {
                    let (current, basis) = &available[identity];
                    if current.value != mapped || current.history != value.history {
                        return Err(Failure::Identity(value.identity));
                    }
                    (current.identity, basis.clone())
                } else {
                    let identity = match resource.entry(*equivalent.first().unwrap()) {
                        std::collections::btree_map::Entry::Occupied(entry) => *entry.get(),
                        std::collections::btree_map::Entry::Vacant(entry) => *entry.insert(
                            occurrence::Identity(take(&mut target.allocation.occurrence)?),
                        ),
                    };
                    (identity, BTreeSet::new())
                };
            origin
                .get_mut(&destination)
                .unwrap()
                .resource
                .insert(value.identity, identity);
            let occurrence = Occurrence {
                identity,
                value: mapped,
                history: value.history.clone(),
            };
            if held
                .insert(identity, occurrence.clone())
                .is_some_and(|previous| previous != occurrence)
            {
                return Err(Failure::Identity(identity));
            }
            flow.resource
                .insert(Place::Held(destination, identity), basis);
        }
        let declaration = original
            .declaration
            .iter()
            .cloned()
            .map(|rule| {
                let Value::Rule(rule) =
                    activation::rename(Value::Rule(Box::new(rule)), &|identity| {
                        Ok(frame[&identity])
                    })?
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
                parent: original.parent.map(|identity| frame[&identity]),
                lexical: original.lexical.map(|identity| frame[&identity]),
                declaration,
                held: held.into_values().collect(),
            },
        );
        flow.frame.insert(destination, None);
    }
    Ok(Archive { frame, origin })
}
