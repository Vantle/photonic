use crate::index::Index;
use crate::term::Term;
use smallvec::SmallVec;
use std::sync::Arc;

pub(super) struct Member {
    pub site: usize,
    pub particle: Option<crate::factor::Cursor>,
}

pub(super) struct Space {
    pub frame: usize,
    pub store: Option<Arc<super::Store>>,
    pub cached: usize,
    pub preparation: Option<crate::plan::Context>,
    pub pattern: Arc<Vec<Vec<Term>>>,
    pub group: Arc<Vec<usize>>,
    pub domain: SmallVec<[Vec<Member>; 2]>,
    pub retained: usize,
}

impl Space {
    pub fn new(
        pattern: Arc<Vec<Vec<Term>>>,
        index: &Index,
        frame: usize,
        preparation: Option<crate::plan::Context>,
        store: Option<Arc<super::Store>>,
    ) -> Self {
        let domain = pattern
            .iter()
            .enumerate()
            .map(|(position, pattern)| {
                preparation
                    .as_ref()
                    .map_or_else(
                        || index.candidate(pattern, frame),
                        |context| context.candidate(position, index, frame),
                    )
                    .into_iter()
                    .map(|world| Member {
                        site: index.site(world),
                        particle: None,
                    })
                    .collect()
            })
            .collect();
        let group = preparation.as_ref().map_or_else(
            || Arc::new(crate::partition::classify(&pattern)),
            crate::plan::Context::group,
        );
        let domain: SmallVec<[Vec<Member>; 2]> = domain;
        let retained = pattern.iter().map(Vec::len).sum::<usize>()
            + domain.iter().map(|member| member.len() + 1).sum::<usize>();
        Self {
            frame,
            store,
            cached: 0,
            preparation,
            pattern,
            group,
            domain,
            retained,
        }
    }

    pub fn update(&mut self, index: &Index) -> SmallVec<[usize; 2]> {
        let mut changed = SmallVec::new();
        for (position, domain) in self.domain.iter_mut().enumerate() {
            if domain.len() > 16
                && self
                    .preparation
                    .as_ref()
                    .is_some_and(|context| !context.affected(position, index, self.frame))
            {
                continue;
            }
            let previous = domain.len();
            domain.retain(|member| {
                if !index.removal.contains(&member.site) {
                    return true;
                }
                self.retained -= 1;
                if let Some(particle) = &member.particle {
                    self.retained -= particle.retained();
                    self.cached -= particle.cached();
                }
                false
            });
            let mut affected = previous != domain.len();
            for &site in &index.insertion {
                let world = &index.state.world[index.world(site)];
                if world.frame != self.frame {
                    continue;
                }
                let eligible = self.preparation.as_ref().map_or_else(
                    || {
                        self.pattern[position]
                            .iter()
                            .all(|term| world.particle.iter().any(|token| term.matches(token)))
                    },
                    |context| context.matches(position, index, site),
                );
                if eligible {
                    affected = true;
                    domain.push(Member {
                        site,
                        particle: None,
                    });
                    self.retained += 1;
                }
            }
            if affected {
                changed.push(position);
            }
        }
        changed
    }

    pub fn prepare(
        &mut self,
        position: usize,
        candidate: usize,
        index: &Index,
    ) -> &mut crate::factor::Cursor {
        let member = &mut self.domain[position][candidate];
        if member.particle.is_none() {
            let particle = &index.state.world[index.world(member.site)].particle;
            let particle = if let Some(context) = &self.preparation {
                context.select(position, index, member.site)
            } else {
                crate::particle::Match::new(&self.pattern[position], particle)
            };
            self.retained += particle.retained();
            let budget = self
                .store
                .as_ref()
                .filter(|_| self.pattern[position].len() >= 8)
                .map(|store| store.budget().clone());
            member.particle = Some(crate::factor::Cursor::new(particle, budget));
        }
        member.particle.as_mut().unwrap()
    }

    pub fn evict(&mut self) {
        if self.cached == 0 {
            return;
        }
        for particle in self
            .domain
            .iter_mut()
            .flatten()
            .filter_map(|member| member.particle.as_mut())
        {
            particle.evict();
        }
        self.retained -= self.cached;
        self.cached = 0;
    }

    #[cfg(test)]
    pub fn size(&self) -> usize {
        self.pattern.iter().map(Vec::len).sum::<usize>()
            + self
                .domain
                .iter()
                .map(|domain| {
                    1 + domain
                        .iter()
                        .map(|member| {
                            1 + member
                                .particle
                                .as_ref()
                                .map_or(0, crate::factor::Cursor::retained)
                        })
                        .sum::<usize>()
                })
                .sum::<usize>()
    }

    pub fn retained(&self) -> usize {
        self.retained
    }
}
