use super::Match;
use crate::program::Symbol;
use crate::state::Token;
use crate::term::Term;
use std::collections::BTreeSet;
use std::task::Poll;

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
            loop {
                match search.step() {
                    Poll::Ready(Some(value)) => assert!(actual.insert(value)),
                    Poll::Ready(None) => break,
                    Poll::Pending => {}
                }
            }
            assert_eq!(actual, expected);
            search.reset();
        }
    }
}
