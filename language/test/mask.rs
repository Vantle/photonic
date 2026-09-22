use super::Set;
use std::collections::BTreeSet;

fn verify(actual: &Set, expected: &BTreeSet<usize>) {
    assert_eq!(actual.len(), expected.len());
    assert_eq!(actual.is_empty(), expected.is_empty());
    assert!(actual.iter().eq(expected.iter().copied()));
}

#[test]
fn reconciliation() {
    let mut actual = Set::default();
    let mut other = Set::default();
    let mut expected = BTreeSet::new();
    let mut complement = BTreeSet::new();
    let mut seed = 29u64;
    for iteration in 0..8192 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let value = (seed >> 33) as usize % 197;
        match iteration % 7 {
            0..=2 => assert_eq!(actual.insert(value), expected.insert(value)),
            3..=4 => assert_eq!(actual.remove(value), expected.remove(&value)),
            5 => assert_eq!(other.insert(value), complement.insert(value)),
            _ => {
                assert_eq!(other.remove(value), complement.remove(&value));
                if iteration % 3 == 0 {
                    let mut union = actual.clone();
                    union.union(&other);
                    verify(&union, &expected.union(&complement).copied().collect());
                }
            }
        }
        assert_eq!(actual.contains(value), expected.contains(&value));
        verify(&actual, &expected);
        verify(&other, &complement);
        if iteration % 1024 == 1023 {
            actual.clear();
            expected.clear();
            verify(&actual, &expected);
        }
    }
    assert!(Set::default().iter().next().is_none());
    assert!(!Set::default().contains(4096));
    assert!(!Set::default().remove(4096));
    let boundary = [0, 63, 64, 127, 128, 4095].into_iter().collect::<Set>();
    assert!(boundary.iter().eq([0, 63, 64, 127, 128, 4095]));
}
