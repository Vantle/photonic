use super::Set;
use std::collections::BTreeSet;

#[test]
fn mutation() {
    let empty = Set::new(0);
    assert_eq!(empty.len(), 0);
    assert_eq!(empty.iter().next(), None);
    for width in [1, 2, 63, 64, 65, 127, 128, 129, 4096, 65537] {
        let mut actual = Set::new(width);
        let mut expected = BTreeSet::new();
        for value in [0, width / 2, width - 1] {
            assert_eq!(actual.insert(value), expected.insert(value));
        }
        let mut seed = 731u64;
        for iteration in 0..2048 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let value = (seed >> 32) as usize % width;
            if iteration % 3 == 0 {
                assert_eq!(actual.remove(&value), expected.remove(&value));
            } else {
                assert_eq!(actual.insert(value), expected.insert(value));
            }
            assert_eq!(actual.contains(&value), expected.contains(&value));
            assert_eq!(actual.len(), expected.len());
            assert_eq!(
                actual.iter().collect::<Vec<_>>(),
                expected.iter().copied().collect::<Vec<_>>()
            );
        }
        for value in expected {
            assert!(actual.remove(&value));
            assert!(!actual.remove(&value));
        }
        assert_eq!(actual.len(), 0);
        assert_eq!(actual.iter().next(), None);
        assert!(!actual.contains(&width));
        assert!(!actual.remove(&usize::MAX));
    }
}
