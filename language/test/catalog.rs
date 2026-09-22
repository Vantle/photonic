use super::scope::Scope;
use std::collections::BTreeMap;

#[test]
fn grouping() {
    for width in [0, 1, 2, 63, 64, 65, 127, 128, 4096] {
        let input = (0..width)
            .map(|position| position * 31 % 257)
            .collect::<Vec<_>>();
        let rule = (0..width).rev().chain(0..width).collect::<Vec<_>>();
        let mut expected = BTreeMap::<_, Vec<_>>::new();
        for &rule in &rule {
            expected.entry(input[rule]).or_default().push(rule);
        }
        let actual = Scope::new(&rule, &input);
        assert_eq!(actual.len(), expected.len());
        assert_eq!(actual.retained(), expected.len() + rule.len());
        assert_eq!(
            actual.iter().collect::<Vec<_>>(),
            expected.keys().copied().collect::<Vec<_>>()
        );
        for (position, (&input, rule)) in expected.iter().enumerate() {
            assert_eq!(actual.get(position), (input, rule.as_slice()));
            assert_eq!(actual.position(input), Some(position));
        }
        assert_eq!(actual.position(257), None);
    }
}
