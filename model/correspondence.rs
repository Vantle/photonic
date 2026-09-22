use crate::archive::Origin;
use crate::comparison::Mapping;
use crate::context;
use crate::derivation::Derivation;
use crate::path::Path;
use std::collections::BTreeMap;

struct Entry<'a> {
    identity: context::Identity,
    origin: &'a Origin,
    target: &'a BTreeMap<context::Identity, Origin>,
}

fn extend(
    source: &Origin,
    target: &Origin,
    derivation: &Derivation,
    mut mapping: Mapping,
) -> Option<Mapping> {
    let original = derivation.at(&source.address)?;
    for (identity, copy) in &source.resource {
        let identity = original.occurrence().get(identity)?;
        let destination = target.resource.get(identity)?;
        mapping = mapping.extend(BTreeMap::new(), BTreeMap::from([(*copy, *destination)]))?;
    }
    crate::provenance::origin(original, &mapping, source, target).then_some(mapping)
}

fn search(
    entry: &[Entry<'_>],
    derivation: &Derivation,
    mapping: &Mapping,
    accept: &mut dyn FnMut(&Mapping) -> bool,
) -> bool {
    let Some((source, remaining)) = entry.split_first() else {
        return accept(mapping);
    };
    for (&identity, target) in source.target {
        let Some(next) = mapping.extend(
            BTreeMap::from([(source.identity, identity)]),
            BTreeMap::new(),
        ) else {
            continue;
        };
        let Some(next) = extend(source.origin, target, derivation, next) else {
            continue;
        };
        if search(remaining, derivation, &next, accept) {
            return true;
        }
    }
    false
}

pub(crate) fn find(
    left: &Path,
    right: &Path,
    derivation: &Derivation,
    accept: &mut dyn FnMut(&Mapping) -> bool,
) -> bool {
    let mut entry = Vec::new();
    for (source, target) in left.record().iter().zip(right.record()) {
        if source.archive.len() != target.archive.len() {
            return false;
        }
        for (&identity, origin) in &source.archive {
            entry.push(Entry {
                identity,
                origin,
                target: &target.archive,
            });
        }
    }
    search(&entry, derivation, derivation.identity(), accept)
}
