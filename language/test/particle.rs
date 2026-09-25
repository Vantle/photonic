use super::Match;
use crate::program::Symbol;
use crate::state::Token;
use crate::term::Term;
use std::collections::BTreeSet;

fn enumerate(
    pattern: &[Term],
    particle: &[Token],
    prefix: &mut Vec<usize>,
    result: &mut BTreeSet<Vec<usize>>,
) {
    let Some(term) = pattern.get(prefix.len()) else {
        result.insert(prefix.clone());
        return;
    };
    for token in particle {
        if token.value != term.value
            || matches!(term.value, Symbol::Rule(_)) && token.capture != term.capture
            || prefix.contains(&token.id)
            || prefix
                .iter()
                .enumerate()
                .any(|(position, &id)| pattern[position] == *term && id >= token.id)
        {
            continue;
        }
        prefix.push(token.id);
        enumerate(pattern, particle, prefix, result);
        prefix.pop();
    }
}

#[test]
fn combination() {
    let mut seed = 59u64;
    let mut next = |bound| {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (seed >> 32) as usize % bound
    };
    for _ in 0..512 {
        let mut particle = (0..next(7))
            .map(|id| {
                let value = if next(2) == 0 {
                    Symbol::Atom(next(2))
                } else {
                    Symbol::Rule(0)
                };
                Token {
                    id,
                    value,
                    capture: matches!(value, Symbol::Rule(_)).then(|| next(2)),
                }
            })
            .collect::<Vec<_>>();
        if let Some(token) = particle.first().cloned() {
            particle.push(token);
        }
        let pattern = (0..next(5))
            .map(|_| {
                if next(2) == 0 {
                    Term::new(Symbol::Atom(next(2)), None)
                } else {
                    Term::new(Symbol::Rule(0), Some(next(2)))
                }
            })
            .collect::<Vec<_>>();
        let mut expected = BTreeSet::new();
        enumerate(&pattern, &particle, &mut Vec::new(), &mut expected);
        let mut search = Match::new(&pattern, &particle);
        for _ in 0..3 {
            let mut actual = BTreeSet::new();
            while let Some(value) = search.step() {
                assert!(actual.insert(value));
            }
            assert_eq!(actual, expected);
            search.reset();
        }
    }
}

#[test]
fn preparation() {
    for capture in [None, Some(0), Some(1)] {
        for encoding in 0..256usize {
            let pattern = (0..4)
                .map(|position| match (encoding >> (position * 2)) & 3 {
                    0 => Symbol::Atom(0),
                    1 => Symbol::Atom(1),
                    _ => Symbol::Rule(0),
                })
                .collect::<Vec<_>>();
            let fragment = crate::pattern::Pattern::new(&pattern);
            let pattern = pattern
                .iter()
                .map(|&value| Term::new(value, capture))
                .collect::<Vec<_>>();
            let mut particle = (0..8)
                .map(|id| Token {
                    id,
                    value: match id % 3 {
                        0 => Symbol::Atom(0),
                        1 => Symbol::Atom(1),
                        _ => Symbol::Rule(0),
                    },
                    capture: (id % 3 == 2).then_some(id % 2),
                })
                .collect::<Vec<_>>();
            particle.push(particle[0].clone());
            let mut expected = BTreeSet::new();
            enumerate(&pattern, &particle, &mut Vec::new(), &mut expected);
            let mut prepared = Match::prepared(&fragment, capture, &particle);
            let mut reference = Match::new(&pattern, &particle);
            for _ in 0..3 {
                let mut actual = BTreeSet::new();
                loop {
                    let value = prepared.step();
                    assert_eq!(value, reference.step());
                    assert_eq!(prepared.retained(), reference.retained());
                    match value {
                        Some(value) => assert!(actual.insert(value)),
                        None => break,
                    }
                }
                assert_eq!(actual, expected);
                prepared.reset();
                reference.reset();
            }
        }
    }
}

#[test]
fn indexed() {
    for width in [8, 16, 32] {
        for capture in [None, Some(0), Some(1)] {
            let value = |index| {
                if index % 2 == 0 {
                    Symbol::Atom(index)
                } else {
                    Symbol::Rule(index)
                }
            };
            let pattern = (0..width).map(value).collect::<Vec<_>>();
            let fragment = crate::pattern::Pattern::new(&pattern);
            let pattern = pattern
                .iter()
                .map(|&value| Term::new(value, capture))
                .collect::<Vec<_>>();
            let mut particle = (0..256)
                .map(|id| Token {
                    id,
                    value: value(id % 64),
                    capture: Some(id / 64 % 2),
                })
                .collect::<Vec<_>>();
            particle.push(particle[0].clone());
            particle.reverse();
            let mut actual = Match::prepared(&fragment, capture, &particle);
            let compiled = crate::pattern::Pattern::new(&pattern);
            let mut specialized = Match::compiled(&compiled, &particle);
            let mut expected = Match::new(&pattern, &particle);
            for _ in 0..3 {
                for _ in 0..1024 {
                    let result = expected.step();
                    assert_eq!(actual.step(), result);
                    assert_eq!(specialized.step(), result);
                    assert_eq!(actual.retained(), expected.retained());
                    assert_eq!(specialized.retained(), expected.retained());
                }
                actual.reset();
                specialized.reset();
                expected.reset();
            }
        }
    }
}
