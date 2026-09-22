use super::Set;
use crate::program::Symbol;
use crate::state::Token;

fn population(width: usize) -> Vec<Token> {
    (0..width)
        .map(|identity| Token {
            id: identity,
            value: Symbol::Rule(identity % 4),
            capture: Some(identity % 3),
        })
        .collect()
}

fn verify(actual: &Set, expected: &[Token]) {
    assert_eq!(actual.len(), expected.len());
    assert_eq!(actual.is_empty(), expected.is_empty());
    assert!(actual.iter().eq(expected.iter()));
    let mut cursor = actual.entry();
    for (offset, token) in expected.iter().enumerate() {
        assert_eq!(
            cursor.size_hint(),
            (expected.len() - offset, Some(expected.len() - offset))
        );
        let (position, value) = cursor.next().unwrap();
        assert_eq!(value, token);
        assert_eq!(actual.at(position), token);
        assert_eq!(&actual[offset], token);
    }
    assert_eq!(cursor.len(), 0);
    assert_eq!(cursor.next(), None);
    assert_eq!(cursor.next(), None);
    let rebuilt = Set::from(expected.to_vec());
    assert_eq!(actual, &rebuilt);
    assert_eq!(actual.cmp(&rebuilt), std::cmp::Ordering::Equal);
    assert_eq!(
        crate::hashing::value(actual),
        crate::hashing::value(&rebuilt)
    );
}

#[test]
fn persistence() {
    for width in [0, 1, 2, 63, 64, 65, 127, 128, 129, 512, 4096] {
        let original = population(width);
        let root = Set::from(original.clone());
        let mut actual = root.clone();
        let mut expected = original.clone();
        verify(&actual, &expected);
        for divisor in [2, 3, 7, 11, 17] {
            let previous = actual.clone();
            let snapshot = expected.clone();
            let mut visited = Vec::new();
            actual.retain(|token| {
                visited.push(token.id);
                !token.id.is_multiple_of(divisor)
            });
            expected.retain(|token| !token.id.is_multiple_of(divisor));
            assert_eq!(
                visited,
                snapshot.iter().map(|token| token.id).collect::<Vec<_>>()
            );
            assert_eq!(
                previous.removed(&actual),
                snapshot
                    .iter()
                    .filter(|token| token.id.is_multiple_of(divisor))
                    .map(|token| token.id)
                    .collect::<Vec<_>>()
            );
            assert!(actual.removed(&previous).is_empty());
            assert_eq!(
                actual
                    .entry()
                    .map(|(position, token)| (position, token.id))
                    .collect::<Vec<_>>(),
                expected
                    .iter()
                    .map(|token| (token.id, token.id))
                    .collect::<Vec<_>>()
            );
            verify(&actual, &expected);
            verify(&previous, &snapshot);
            verify(&root, &original);
        }
        let snapshot = actual.clone();
        actual.retain(|_| true);
        assert!(actual.removed(&snapshot).is_empty());
        assert_eq!(actual, snapshot);
        actual.clear();
        verify(&actual, &[]);
        verify(&snapshot, &expected);
    }
}

#[test]
fn branch() {
    for width in [0, 1, 63, 64, 65, 128, 129, 513] {
        let original = population(width);
        let root = Set::from(original.clone());
        let mut left = root.clone();
        let mut right = root.clone();
        left.retain(|token| token.id % 3 == 0);
        right.retain(|token| token.id % 2 == 0);
        assert_eq!(
            left.removed(&right),
            (0..width)
                .filter(|identity| *identity % 3 == 0 && *identity % 2 != 0)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            right.removed(&left),
            (0..width)
                .filter(|identity| *identity % 2 == 0 && *identity % 3 != 0)
                .collect::<Vec<_>>()
        );
        let rebuilt = Set::from(left.iter().cloned().collect::<Vec<_>>());
        assert_eq!(
            left.removed(&rebuilt),
            left.entry()
                .map(|(position, _)| position)
                .collect::<Vec<_>>()
        );
        verify(&root, &original);
    }
}

#[test]
fn replacement() {
    let root = Set::from(population(129));
    let mut branch = root.clone();
    branch.retain(|token| token.id % 2 == 1);
    let snapshot = branch.clone();
    branch[0].capture = Some(99);
    branch[1].id = 1000;
    assert!(!branch.shared(&root));
    assert!(!branch.shared(&snapshot));
    assert_eq!(snapshot[0].capture, Some(1));
    assert_eq!(snapshot[1].id, 3);
    assert_eq!(root.len(), 129);
    assert_eq!(branch.len(), 64);
    assert_eq!(
        branch
            .entry()
            .map(|(position, _)| position)
            .collect::<Vec<_>>(),
        (0..64).collect::<Vec<_>>()
    );
    verify(&root, &population(129));
}

#[test]
fn equality() {
    let token = population(1).pop().unwrap();
    let root = Set::from(vec![token.clone(), token]);
    let mut left = root.clone();
    let mut right = root;
    let mut position = 0;
    left.retain(|_| {
        position += 1;
        position == 1
    });
    position = 0;
    right.retain(|_| {
        position += 1;
        position == 2
    });
    assert_eq!(left, right);
    assert_eq!(left.cmp(&right), std::cmp::Ordering::Equal);
    assert_eq!(crate::hashing::value(&left), crate::hashing::value(&right));
    assert_ne!(
        left.entry().next().unwrap().0,
        right.entry().next().unwrap().0
    );
}

#[test]
fn capture() {
    let owned = (0..130)
        .map(|identity| Token {
            id: identity,
            value: Symbol::Rule(identity % 4),
            capture: Some(5),
        })
        .collect::<Vec<_>>();
    let mut uniform = Set::from(owned.clone());
    assert_eq!(uniform.capture(), Some(5));
    uniform.retain(|token| token.id % 3 == 0);
    assert_eq!(uniform.capture(), Some(5));
    assert!(uniform.iter().all(|token| token.capture == Some(5)));
    uniform.clear();
    assert_eq!(uniform.capture(), None);
    assert_eq!(Set::default().capture(), None);
    assert_eq!(Set::from(population(3)).capture(), None);
    let mut mutated = Set::from(owned);
    mutated[7].capture = Some(6);
    assert_eq!(mutated.capture(), None);
}
