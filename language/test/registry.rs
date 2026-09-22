use super::{Key, Registry};
use std::collections::BTreeMap;

#[test]
fn reconciliation() {
    let mut actual = Registry::new(0);
    let mut expected = BTreeMap::new();
    let mut seed = 71u64;
    for iteration in 0..4096 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let frame = (seed >> 32) as usize % 17;
        actual.resize(frame + 1);
        let key = Key {
            frame,
            input: if iteration % 17 == 0 {
                usize::MAX
            } else {
                (seed >> 16) as usize % 11
            },
            owner: if iteration % 19 == 0 {
                usize::MAX
            } else {
                seed as usize % 5
            },
        };
        if iteration % 3 != 0 {
            actual.insert(key, iteration);
            expected.insert(key, iteration);
        } else {
            let interval = if iteration % 2 == 0 {
                Key::frame(frame)
            } else {
                Key::input(frame, key.input)
            };
            assert!(
                actual.range(interval.clone()).eq(expected
                    .range(interval.clone())
                    .map(|(&key, value)| (key, value)))
            );
            let before = expected
                .extract_if(interval.clone(), |_, position| *position % 2 == 0)
                .collect::<Vec<_>>();
            let after = actual
                .extract(interval, |_, position| *position % 2 == 0)
                .collect::<Vec<_>>();
            assert!(before == after);
        }
        assert_eq!(actual.len(), expected.len());
        assert_eq!(actual.get(&key), expected.get(&key));
        assert_eq!(
            actual.count(frame),
            expected.range(Key::frame(frame)).count()
        );
        assert!(
            actual
                .iter()
                .eq(expected.iter().map(|(&key, value)| (key, value)))
        );
        assert!(actual.values().eq(expected.values()));
    }
}

#[test]
fn extraction() {
    for (width, maximum) in [8, 32, 33, 96, 99, 384, 387, 512]
        .into_iter()
        .flat_map(|width| [0, 1, 7, 128].map(|maximum| (width, maximum)))
    {
        let mut actual = Registry::new(3);
        let mut expected = BTreeMap::new();
        for position in (0..width).rev() {
            let key = Key {
                frame: position % 3,
                input: position / 8,
                owner: position % 8,
            };
            actual.insert(key, position);
            expected.insert(key, position);
        }
        for frame in 0..3 {
            for interval in [Key::input(frame, 17), Key::frame(frame)] {
                let predicate = |_: &Key, position: &mut usize| {
                    *position += 1;
                    !position.is_multiple_of(3)
                };
                let before = expected
                    .extract_if(interval.clone(), predicate)
                    .take(maximum)
                    .collect::<Vec<_>>();
                let after = actual
                    .extract(interval, predicate)
                    .take(maximum)
                    .collect::<Vec<_>>();
                assert_eq!(before.len(), after.len());
                assert!(before == after);
                assert_eq!(actual.len(), expected.len());
                assert!(
                    actual
                        .iter()
                        .eq(expected.iter().map(|(&key, value)| (key, value)))
                );
                assert_eq!(
                    actual.count(frame),
                    expected.range(Key::frame(frame)).count()
                );
            }
        }
    }
}
