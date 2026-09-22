use std::collections::BTreeMap;

pub(crate) fn search<Identity: Copy + Ord>(
    candidate: &BTreeMap<Identity, Vec<Identity>>,
    mapping: BTreeMap<Identity, Identity>,
    valid: &impl Fn(&BTreeMap<Identity, Identity>) -> bool,
    accept: &mut impl FnMut(&BTreeMap<Identity, Identity>) -> bool,
) -> bool {
    if !valid(&mapping) {
        return false;
    }
    let selected = candidate
        .iter()
        .filter(|(source, _)| !mapping.contains_key(source))
        .min_by_key(|(_, target)| {
            target
                .iter()
                .filter(|target| !mapping.values().any(|value| value == *target))
                .count()
        });
    let Some((&source, target)) = selected else {
        return accept(&mapping);
    };
    for &target in target {
        if mapping.values().any(|&value| value == target) {
            continue;
        }
        let mut next = mapping.clone();
        next.insert(source, target);
        if search(candidate, next, valid, accept) {
            return true;
        }
    }
    false
}
