mod group;

use crate::state::Token;
use crate::term::Term;
use group::Group;
use smallvec::SmallVec;
use std::sync::Arc;

pub(crate) fn wide(width: usize) -> bool {
    width >= 8
}

pub(crate) struct Preparation {
    group: SmallVec<[Group; 1]>,
    width: usize,
    viable: bool,
}

impl Preparation {
    fn new(group: SmallVec<[Group; 1]>, width: usize) -> Arc<Self> {
        let viable = group
            .iter()
            .all(|group| group.candidate.len() >= group.position.len());
        Arc::new(Self {
            group,
            width,
            viable,
        })
    }

    pub fn retained(&self) -> usize {
        self.group
            .iter()
            .map(|group| group.candidate.len() + group.position.len())
            .sum()
    }
}

pub(crate) struct Match {
    preparation: Arc<Preparation>,
    selected: Box<[usize]>,
    fresh: bool,
    complete: bool,
}

impl From<Arc<Preparation>> for Match {
    fn from(preparation: Arc<Preparation>) -> Self {
        let mut selected = vec![0; preparation.width].into_boxed_slice();
        for group in &preparation.group {
            group.reset(&mut selected);
        }
        Self {
            complete: !preparation.viable,
            preparation,
            selected,
            fresh: true,
        }
    }
}

impl Match {
    pub(crate) fn new(pattern: &[Term], particle: &[Token]) -> Self {
        let mut group: SmallVec<[Group; 1]> = SmallVec::new();
        let mut known: SmallVec<[(&Term, usize); 4]> = SmallVec::new();
        for (position, term) in pattern.iter().enumerate() {
            if let Some(&(_, index)) = known.iter().find(|&&(value, _)| value == term) {
                let group = &mut group[index];
                Arc::make_mut(&mut group.position).push(position);
                continue;
            }
            let mut candidate = particle
                .iter()
                .filter(|token| term.matches(token))
                .map(|token| token.id)
                .collect::<Vec<_>>();
            candidate.sort_unstable();
            candidate.dedup();
            if let Some(index) = group.iter().position(|group| group.candidate == candidate) {
                known.push((term, index));
                let group = &mut group[index];
                Arc::make_mut(&mut group.position).push(position);
            } else {
                known.push((term, group.len()));
                group.push(Group {
                    candidate,
                    position: Arc::new(vec![position]),
                });
            }
        }
        Self::from(Preparation::new(group, pattern.len()))
    }

    pub(crate) fn prepared(
        pattern: &crate::pattern::Pattern,
        capture: Option<usize>,
        particle: &[Token],
    ) -> Self {
        Self::materialize(pattern, particle, |value| Term::new(*value, capture))
    }

    pub(crate) fn compiled(pattern: &crate::pattern::Pattern<Term>, particle: &[Token]) -> Self {
        Self::materialize(pattern, particle, Clone::clone)
    }

    fn materialize<Value>(
        pattern: &crate::pattern::Pattern<Value>,
        particle: &[Token],
        term: impl Fn(&Value) -> Term,
    ) -> Self {
        let mut indexed = (pattern.group.len() >= 8 && particle.len() >= 32).then(|| {
            let mut candidate = pattern
                .group
                .iter()
                .map(|group| (term(&group.value), Vec::new()))
                .collect::<std::collections::HashMap<_, _, crate::hashing::Builder>>();
            for token in particle {
                if let Some(candidate) = candidate.get_mut(&Term::new(token.value, token.capture)) {
                    candidate.push(token.id);
                }
            }
            candidate
        });
        let mut group: SmallVec<[Group; 1]> = SmallVec::with_capacity(pattern.group.len());
        for requirement in &pattern.group {
            let term = term(&requirement.value);
            let mut candidate = indexed.as_mut().map_or_else(
                || {
                    particle
                        .iter()
                        .filter(|token| term.matches(token))
                        .map(|token| token.id)
                        .collect::<Vec<_>>()
                },
                |candidate| candidate.remove(&term).unwrap(),
            );
            candidate.sort_unstable();
            candidate.dedup();
            if let Some(group) = group.iter_mut().find(|group| group.candidate == candidate) {
                Arc::make_mut(&mut group.position).extend(requirement.position.iter().copied());
            } else {
                group.push(Group {
                    candidate,
                    position: requirement.position.clone(),
                });
            }
        }
        Self::from(Preparation::new(group, pattern.width))
    }

    pub(crate) fn impossible() -> Self {
        static EMPTY: std::sync::LazyLock<Arc<Preparation>> = std::sync::LazyLock::new(|| {
            Arc::new(Preparation {
                group: SmallVec::new(),
                width: 0,
                viable: false,
            })
        });
        Self {
            preparation: EMPTY.clone(),
            selected: Box::new([]),
            fresh: true,
            complete: true,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.fresh = true;
        self.complete = !self.preparation.viable;
        for group in &self.preparation.group {
            group.reset(&mut self.selected);
        }
    }

    pub(crate) fn viable(&self) -> bool {
        self.preparation.viable
    }

    pub(crate) fn step(&mut self) -> Option<Vec<usize>> {
        if self.complete {
            return None;
        }
        if !self.fresh
            && !self
                .preparation
                .group
                .iter()
                .rev()
                .any(|group| group.advance(&mut self.selected))
        {
            self.complete = true;
            return None;
        }
        self.fresh = false;
        let mut result = vec![0; self.preparation.width];
        for group in &self.preparation.group {
            for &position in group.position.iter() {
                result[position] = group.candidate[self.selected[position]];
            }
        }
        Some(result)
    }

    pub(crate) fn preparation(&self) -> Arc<Preparation> {
        self.preparation.clone()
    }

    pub(crate) fn retained(&self) -> usize {
        self.preparation.retained() + self.selected.len()
    }
}

#[cfg(test)]
#[path = "test/particle.rs"]
mod test;
