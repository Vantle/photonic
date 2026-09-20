use crate::program::Symbol;
use crate::term::Term;
use smallvec::SmallVec;
use std::sync::Arc;

pub(crate) struct Input {
    fragment: Arc<Vec<Arc<crate::pattern::Pattern>>>,
    group: Arc<Vec<usize>>,
    pattern: Arc<Vec<Vec<Term>>>,
    capture: bool,
    arity: usize,
    empty: bool,
    dependency: SmallVec<[Symbol; 2]>,
}

pub(crate) struct Context {
    fragment: Arc<Vec<Arc<crate::pattern::Pattern>>>,
    group: Arc<Vec<usize>>,
    owner: usize,
}

impl Context {
    pub fn group(&self) -> Arc<Vec<usize>> {
        self.group.clone()
    }

    pub fn affected(&self, position: usize, index: &crate::index::Index, frame: usize) -> bool {
        index.affected.get(&frame).is_some_and(|symbol| {
            self.fragment[position]
                .group
                .iter()
                .all(|group| symbol.contains(&group.value))
        })
    }

    pub fn candidate(
        &self,
        position: usize,
        index: &crate::index::Index,
        frame: usize,
    ) -> Vec<usize> {
        let pattern = self.fragment[position]
            .group
            .iter()
            .map(|group| Term::new(group.value, Some(self.owner)));
        index.candidate(pattern, frame)
    }

    pub fn matches(&self, position: usize, index: &crate::index::Index, site: usize) -> bool {
        let world = &index.state.world[index.world(site)];
        self.fragment[position].group.iter().all(|group| {
            let term = Term::new(group.value, Some(self.owner));
            if world.particle.len() <= 16 {
                world.particle.iter().any(|token| term.matches(token))
            } else {
                index.quantity(&term, world.frame, site) > 0
            }
        })
    }

    pub fn select(
        &self,
        position: usize,
        index: &crate::index::Index,
        site: usize,
        shared: Option<&crate::preparation::Store>,
    ) -> crate::particle::Match {
        let world = &index.state.world[index.world(site)];
        let pattern = &self.fragment[position];
        if pattern.width > 4
            && pattern.group.iter().any(|group| {
                index.quantity(&Term::new(group.value, Some(self.owner)), world.frame, site)
                    < group.position.len()
            })
        {
            return crate::particle::Match::impossible(pattern.width);
        }
        if pattern.width >= 8
            && world.particle.len() >= 32
            && let Some(shared) = shared
        {
            return shared.select(crate::preparation::Request {
                pattern,
                world,
                owner: self.owner,
            });
        }
        self.prepare(position, &world.particle)
    }

    pub fn prepare(
        &self,
        position: usize,
        particle: &[crate::state::Token],
    ) -> crate::particle::Match {
        crate::particle::Match::prepared(&self.fragment[position], Some(self.owner), particle)
    }
}

impl Input {
    #[cfg(test)]
    pub fn new(value: &[Vec<Symbol>]) -> Self {
        Self::shared(value, &mut Default::default())
    }

    pub fn shared(
        value: &[Vec<Symbol>],
        shared: &mut std::collections::HashMap<Vec<Symbol>, Arc<crate::pattern::Pattern>>,
    ) -> Self {
        let pattern = Arc::new(if value.is_empty() {
            vec![Vec::new()]
        } else {
            pattern(value, None)
        });
        let mut dependency = value
            .iter()
            .flatten()
            .copied()
            .collect::<SmallVec<[Symbol; 2]>>();
        dependency.sort_unstable();
        dependency.dedup();
        let capture = dependency
            .iter()
            .any(|symbol| matches!(symbol, Symbol::Rule(_)));
        Self {
            group: Arc::new(crate::partition::classify(&pattern)),
            fragment: Arc::new(
                pattern
                    .iter()
                    .map(|particle| {
                        let value = particle.iter().map(|term| term.value).collect::<Vec<_>>();
                        shared
                            .entry(value.clone())
                            .or_insert_with(|| Arc::new(crate::pattern::Pattern::new(&value)))
                            .clone()
                    })
                    .collect(),
            ),
            pattern,
            capture,
            arity: value.len(),
            empty: value.is_empty() || value.iter().any(Vec::is_empty),
            dependency,
        }
    }

    pub fn owner(&self, owner: usize) -> usize {
        if self.capture { owner } else { 0 }
    }

    pub fn pattern(&self, owner: usize) -> Arc<Vec<Vec<Term>>> {
        if self.capture {
            return Arc::new(
                self.pattern
                    .iter()
                    .map(|particle| {
                        particle
                            .iter()
                            .map(|term| Term::new(term.value, Some(owner)))
                            .collect()
                    })
                    .collect(),
            );
        }
        self.pattern.clone()
    }

    pub fn context(&self, owner: usize) -> Context {
        Context {
            fragment: self.fragment.clone(),
            group: self.group.clone(),
            owner,
        }
    }

    pub fn empty(&self) -> bool {
        self.empty
    }

    pub fn dependency(&self) -> &[Symbol] {
        &self.dependency
    }

    pub fn retained(&self) -> usize {
        self.arity * 2
            + self.pattern.iter().map(Vec::len).sum::<usize>() * 2
            + self.dependency.len()
    }
}

pub(crate) fn pattern(input: &[Vec<Symbol>], capture: Option<usize>) -> Vec<Vec<Term>> {
    input
        .iter()
        .map(|particle| {
            particle
                .iter()
                .map(|&value| Term::new(value, capture))
                .collect()
        })
        .collect()
}

#[cfg(test)]
#[path = "test/plan.rs"]
mod test;
