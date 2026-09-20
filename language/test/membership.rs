#[test]
fn mutation() {
    let mut expected = std::collections::BTreeSet::new();
    let mut actual = crate::membership::Set::default();
    for width in [2, 512, 16] {
        for value in (0..width).rev() {
            expected.insert(value);
            actual.insert(value);
            actual.insert(value);
            assert_eq!(
                actual.iter().collect::<Vec<_>>(),
                expected.iter().collect::<Vec<_>>()
            );
        }
        for value in 0..width {
            expected.remove(&value);
            actual.remove(&value);
            actual.remove(&value);
            assert_eq!(
                actual.iter().collect::<Vec<_>>(),
                expected.iter().collect::<Vec<_>>()
            );
        }
        assert!(actual.is_empty());
    }
}
