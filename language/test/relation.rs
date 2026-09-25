use crate::relation::Map;
use std::collections::BTreeMap;
use std::hash::{DefaultHasher, Hash, Hasher};

fn digest(value: &Map<usize, usize>) -> u64 {
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
                .map(|position| {
                    let key = cursor % 3;
                    cursor /= 3;
                    (key, position as usize)
                })
                .collect::<Vec<_>>();
            let expected = input.iter().copied().collect::<BTreeMap<_, _>>();
            let actual = input.into_iter().collect::<Map<_, _>>();
            let canonical = expected
                .iter()
                .map(|(&key, &value)| (key, value))
                .collect::<Map<_, _>>();
            assert_eq!(
                actual
                    .iter()
                    .map(|(&key, &value)| (key, value))
                    .collect::<Vec<_>>(),
                expected
                    .iter()
                    .map(|(&key, &value)| (key, value))
                    .collect::<Vec<_>>()
            );
            for (key, value) in expected {
                assert_eq!(actual[&key], value);
            }
            assert_eq!(actual, canonical);
            assert_eq!(digest(&actual), digest(&canonical));
        }
    }
}
