#[test]
fn mutation() {
    let mut expected = std::collections::BTreeSet::new();
    let mut actual = crate::membership::Set::default();
    for width in [2, 512, 16] {
        for value in (0..width).rev() {
            assert_eq!(actual.insert(value), expected.insert(value));
            assert!(!actual.insert(value));
            assert_eq!(
                actual.iter().collect::<Vec<_>>(),
                expected.iter().collect::<Vec<_>>()
            );
        }
        for value in 0..width {
            assert_eq!(actual.remove(&value), expected.remove(&value));
            assert!(!actual.remove(&value));
            assert_eq!(
                actual.iter().collect::<Vec<_>>(),
                expected.iter().collect::<Vec<_>>()
            );
        }
        assert!(actual.iter().next().is_none());
    }
}

#[test]
fn isolation() {
    for width in [2, 32, 33, 512] {
        let mut original = crate::membership::Set::default();
        for value in 0..width {
            original.insert(value);
        }
        let mut copied = original.clone();
        for value in 0..width {
            copied.remove(&value);
        }
        assert!(copied.iter().next().is_none());
        assert_eq!(
            (&original).into_iter().copied().collect::<Vec<_>>(),
            (0..width).collect::<Vec<_>>()
        );
    }
}
