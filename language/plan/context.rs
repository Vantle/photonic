use crate::term::Term;
use std::sync::Arc;

pub(crate) struct Context {
    shape: Arc<super::Shape>,
    owner: usize,
}

impl Context {
    pub(super) fn new(shape: Arc<super::Shape>, owner: usize) -> Self {
        Self { shape, owner }
    }

    pub fn count(&self) -> usize {
        self.shape.pattern.len()
    }

    pub fn width(&self, position: usize) -> usize {
        self.shape.pattern[position].len()
    }

    pub fn group(&self) -> &[usize] {
        &self.shape.group
    }

    pub fn pattern(&self, position: usize) -> impl ExactSizeIterator<Item = Term> + '_ {
        self.shape.pattern[position]
            .iter()
            .map(|&value| Term::new(value, Some(self.owner)))
    }

    pub fn affected(&self, position: usize, index: &crate::index::Index, frame: usize) -> bool {
        if index.invalidated(frame) {
            return true;
        }
        index.affected.get(&frame).is_some_and(|symbol| {
            self.shape.fragment[position].group.iter().all(|group| {
                symbol.contains(&group.value)
                    || index
                        .visible(frame, &Term::new(group.value, Some(self.owner)))
                        .next()
                        .is_some()
            })
        })
    }

    fn term(&self, position: usize) -> impl Iterator<Item = Term> + '_ {
        self.shape.fragment[position]
            .group
            .iter()
            .map(|group| Term::new(group.value, Some(self.owner)))
    }

    pub fn candidate(
        &self,
        position: usize,
        index: &crate::index::Index,
        frame: usize,
    ) -> Vec<usize> {
        index.candidate(self.term(position), frame)
    }

    pub fn possible(&self, position: usize, index: &crate::index::Index, frame: usize) -> bool {
        index.possible(self.term(position), frame)
    }

    pub fn matches(&self, position: usize, index: &crate::index::Index, site: usize) -> bool {
        let location = index.location(site);
        if self.shape.fragment[position].width == 0 {
            return location.world().is_some();
        }
        self.shape.fragment[position].group.iter().all(|group| {
            let term = Term::new(group.value, Some(self.owner));
            index.quantity(&term, location.frame(&index.state), site) > 0
        })
    }

    pub fn select(
        &self,
        position: usize,
        index: &crate::index::Index,
        site: usize,
        shared: Option<&crate::preparation::Store>,
    ) -> crate::particle::Match {
        let location = index.location(site);
        if location.world().is_none()
            || self.shape.fragment[position]
                .group
                .iter()
                .any(|group| matches!(group.value, crate::program::Symbol::Rule(_)))
        {
            let particle = index.particle(site, self.pattern(position));
            return self.prepare(position, &particle);
        }
        let world = &index.state.world[index.world(site)];
        let pattern = &self.shape.fragment[position];
        if world.particle.len() < pattern.width
            || (pattern.group.len() < pattern.width
                && pattern.group.iter().any(|group| {
                    let count = group.position.len();
                    if count == 1 {
                        return false;
                    }
                    let term = Term::new(group.value, Some(self.owner));
                    if world.particle.len() <= 16 {
                        return world
                            .particle
                            .iter()
                            .filter(|token| term.matches(token))
                            .take(count)
                            .count()
                            < count;
                    }
                    index.quantity(&term, world.frame, site) < count
                }))
        {
            return crate::particle::Match::impossible();
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
        crate::particle::Match::prepared(&self.shape.fragment[position], Some(self.owner), particle)
    }
}
