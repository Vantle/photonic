use crate::comparison::{self, Mapping};
use crate::path::{Path, Step};
use crate::provenance;
use crate::support::Address;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Derivation {
    pub(crate) state: Vec<Mapping>,
    pub(crate) branch: BTreeMap<usize, Self>,
    pub(crate) identity: Mapping,
}

impl Derivation {
    pub fn state(&self) -> &[Mapping] {
        &self.state
    }
    pub fn branch(&self) -> &BTreeMap<usize, Self> {
        &self.branch
    }

    pub fn at(&self, address: &Address) -> Option<&Mapping> {
        let mut cursor = self;
        for position in &address.derivation {
            cursor = cursor.branch.get(position)?;
        }
        cursor.state.get(address.state)
    }

    pub fn identity(&self) -> &Mapping {
        &self.identity
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Side {
    Left,
    Right,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Unsupported {
    pub side: Side,
    pub address: Address,
    pub context: crate::context::Identity,
}

fn domain(path: &Path, side: Side, derivation: Vec<usize>) -> Result<(), Unsupported> {
    let frame = path
        .state()
        .iter()
        .flat_map(|state| state.frame().map(|frame| frame.identity))
        .collect::<std::collections::BTreeSet<_>>();
    for (state, record) in path.record().iter().enumerate() {
        for &identity in &record.context {
            if !frame.contains(&identity) && !record.archive.contains_key(&identity) {
                return Err(Unsupported {
                    side,
                    address: Address {
                        derivation,
                        state: state + 1,
                    },
                    context: identity,
                });
            }
        }
        if let Step::Inference { path, .. } = &record.step {
            let mut nested = derivation.clone();
            nested.push(state);
            domain(path, side, nested)?;
        }
    }
    Ok(())
}

fn branch(
    left: &Path,
    right: &Path,
    position: usize,
    mapping: Derivation,
    accept: &mut dyn FnMut(&Derivation) -> bool,
) -> bool {
    if position == left.record().len() {
        return crate::correspondence::find(left, right, &mapping, &mut |identity| {
            let mut completed = mapping.clone();
            completed.identity = identity.clone();
            crate::attestation::check(left, right, &completed) && accept(&completed)
        });
    }
    match (
        &left.record()[position].step,
        &right.record()[position].step,
    ) {
        (Step::Inference { path: source, .. }, Step::Inference { path: target, .. }) => {
            search(source, target, &mut |child| {
                if child.state[0] != mapping.state[position] {
                    return false;
                }
                let mut next = mapping.clone();
                next.branch.insert(position, child.clone());
                branch(left, right, position + 1, next, accept)
            })
        }
        (Step::Inference { .. }, _) | (_, Step::Inference { .. }) => false,
        _ => branch(left, right, position + 1, mapping, accept),
    }
}

fn state(
    left: &Path,
    right: &Path,
    mapping: Vec<Mapping>,
    accept: &mut dyn FnMut(&Derivation) -> bool,
) -> bool {
    let position = mapping.len();
    if position == left.state().len() {
        return branch(
            left,
            right,
            0,
            Derivation {
                identity: mapping.iter().fold(Mapping::default(), |result, mapping| {
                    result.join(mapping).unwrap()
                }),
                state: mapping,
                branch: BTreeMap::new(),
            },
            accept,
        );
    }
    comparison::find(
        &left.state()[position],
        &right.state()[position],
        |candidate| {
            if mapping
                .iter()
                .any(|previous| !provenance::persistent(previous, candidate))
            {
                return false;
            }
            if position > 0
                && (!provenance::compare(
                    &mapping[position - 1],
                    candidate,
                    &left.record()[position - 1].flow,
                    &right.record()[position - 1].flow,
                ) || !provenance::compare(
                    &mapping[0],
                    candidate,
                    left.prefix(position).unwrap(),
                    right.prefix(position).unwrap(),
                ))
            {
                return false;
            }
            let mut next = mapping.clone();
            next.push(candidate.clone());
            state(left, right, next, accept)
        },
    )
    .is_some()
}

fn search(left: &Path, right: &Path, accept: &mut dyn FnMut(&Derivation) -> bool) -> bool {
    if left.state().len() != right.state().len() {
        return false;
    }
    state(left, right, Vec::new(), accept)
}

pub fn compare(left: &Path, right: &Path) -> Result<Option<Derivation>, Unsupported> {
    find(left, right, |_| true)
}

pub fn find(
    left: &Path,
    right: &Path,
    mut accept: impl FnMut(&Derivation) -> bool,
) -> Result<Option<Derivation>, Unsupported> {
    domain(left, Side::Left, Vec::new())?;
    domain(right, Side::Right, Vec::new())?;
    let mut result = None;
    search(left, right, &mut |mapping| {
        if !accept(mapping) {
            return false;
        }
        result = Some(mapping.clone());
        true
    });
    Ok(result)
}
