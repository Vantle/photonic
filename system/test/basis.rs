use crate::basis::Set;
use std::collections::BTreeSet;
use std::hash::{DefaultHasher, Hash, Hasher};

fn digest(value: &Set<usize>) -> u64 {
    let mut state = DefaultHasher::new();
    value.hash(&mut state);
    state.finish()
}

#[test]
fn canonical() {
    for length in 0..=7 {
        for encoding in 0..3usize.pow(length) {
            let mut cursor = encoding;
            let input = (0..length)
                .map(|_| {
                    let value = cursor % 3;
                    cursor /= 3;
                    value
                })
                .collect::<Vec<_>>();
            let expected = input.iter().copied().collect::<BTreeSet<_>>();
            let actual = input.into_iter().collect::<Set<_>>();
            let canonical = Set::from(expected.clone());
            assert_eq!(
                actual.iter().copied().collect::<Vec<_>>(),
                expected.iter().copied().collect::<Vec<_>>()
            );
            assert_eq!(actual.len(), expected.len());
            assert_eq!(actual.first(), expected.first());
            assert_eq!(actual, canonical);
            assert_eq!(digest(&actual), digest(&canonical));
            assert_eq!(actual.clone(), actual);
        }
    }
    assert_eq!(Set::<usize>::default(), [].into_iter().collect());
    assert_eq!(Set::single(4), [4, 4].into_iter().collect());
    assert_eq!(
        digest(&Set::single(4)),
        digest(&[4, 4].into_iter().collect())
    );
}
