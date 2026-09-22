use super::Set;
use crate::program::Symbol;
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn reconciliation() {
    let mut actual = Set::default();
    let mut seed = 43u64;
    for _ in 0..512 {
        let mut expected = BTreeMap::<usize, BTreeSet<Symbol>>::new();
        actual.clear();
        for _ in 0..(seed >> 60) {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let frame = (seed >> 40) as usize % 5;
            let symbol = (0..(seed >> 58) as usize)
                .map(|offset| match (seed >> (offset * 3)) % 3 {
                    0 => Symbol::Atom((seed >> (offset * 5)) as usize % 7),
                    _ => Symbol::Rule((seed >> (offset * 7)) as usize % 4),
                })
                .collect::<Vec<_>>();
            expected
                .entry(frame)
                .or_default()
                .extend(symbol.iter().copied());
            actual.insert(frame, symbol);
        }
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        actual.seal();
        assert!(actual.frame().iter().copied().eq(expected.keys().copied()));
        assert_eq!(
            actual.retained(),
            expected.len() + expected.values().map(BTreeSet::len).sum::<usize>()
        );
        for frame in 0..6 {
            let group = expected.get(&frame);
            assert_eq!(actual.contains(frame), group.is_some());
            assert!(
                actual
                    .symbol(frame)
                    .eq(group.into_iter().flatten().copied())
            );
            for symbol in (0..8).map(Symbol::Atom).chain((0..5).map(Symbol::Rule)) {
                assert_eq!(
                    actual.includes(frame, symbol),
                    group.is_some_and(|group| group.contains(&symbol))
                );
            }
        }
    }
}
