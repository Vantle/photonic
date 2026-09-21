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
    shared: Option<Arc<crate::preparation::Store>>,
    subscription: Box<[(usize, Arc<crate::candidate::Node>)]>,
    pub retained: usize,
}

impl Space {
    pub fn dependency(
        &self,
        site: usize,
        binding: &[crate::slot::Slot],
        order: &[usize],
        index: &Index,
    ) -> super::dependency::Dependency {
        super::dependency::Dependency::new(
            site,
            binding
                .iter()
                .filter(|slot| {
                    let world = &index.state.world[index.world(slot.world)];
                    order[binding.len()..].iter().any(|&position| {
                        self.pattern[position]
                            .iter()
                            .all(|term| world.particle.iter().any(|token| term.matches(token)))
                    })
                })
                .map(|slot| (slot.position, slot.world)),
        )
    }

    pub fn new(
        pattern: Arc<Vec<Vec<Term>>>,
        index: &Index,
        frame: usize,
        preparation: Option<crate::plan::Context>,
        store: Option<&Arc<super::Store>>,
    ) -> Self {
        let mut subscription = Vec::new();
        let domain = pattern
            .iter()
            .enumerate()
            .map(|(position, pattern)| {
                let shared = store.filter(|_| index.state.world.len() > 32 && pattern.len() > 1);
                let selected = if let Some(store) = shared {
                    store.domain.select(crate::candidate::Request {
                        pattern,
                        index,
                        frame,
                    })
                } else {
                    crate::candidate::Domain {
                        site: preparation.as_ref().map_or_else(
                            || index.candidate(pattern.iter().cloned(), frame),
                            |context| context.candidate(position, index, frame),
                        ),
                        node: None,
                    }
                };
                if let Some(node) = selected.node {
                    subscription.push((position, node));
                }
                selected
                    .site
                    .into_iter()
                    .map(|site| Member {
                        site,
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
        let retained = subscription.len() * 2
            + pattern.iter().map(Vec::len).sum::<usize>()
            + domain.iter().map(|member| member.len() + 1).sum::<usize>();
        let shared = store
            .filter(|_| pattern.iter().any(|particle| particle.len() >= 8))
            .map(|store| store.preparation.clone());
        let store = store
            .filter(|_| pattern.len() > 1 && pattern.iter().any(|particle| particle.len() >= 8))
            .cloned();
        Self {
            frame,
            store,
            shared,
            cached: 0,
            preparation,
            pattern,
            group,
            domain,
            subscription: subscription.into_boxed_slice(),
            retained,
        }
    }

    pub fn update(&mut self, index: &Index) -> SmallVec<[usize; 2]> {
        if self.subscription.is_empty() {
            return self.advance::<false>(index);
        }
        self.advance::<true>(index)
    }

    fn advance<const SHARED: bool>(&mut self, index: &Index) -> SmallVec<[usize; 2]> {
        let mut changed = SmallVec::new();
        for (position, domain) in self.domain.iter_mut().enumerate() {
            let change = if SHARED {
                self.subscription
                    .iter()
                    .find(|(input, _)| *input == position)
                    .filter(|(_, node)| Arc::strong_count(node) > 2)
                    .map(|(_, node)| node.change(index))
            } else {
                None
            };
            if domain.len() > 16
                && change.as_ref().map_or_else(
                    || {
                        self.preparation
                            .as_ref()
                            .is_some_and(|context| !context.affected(position, index, self.frame))
                    },
                    |change| !change.affected,
                )
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
            let candidate = change
                .as_ref()
                .map_or(index.insertion.as_slice(), |change| {
                    change.insertion.site.as_slice()
                });
            for &site in candidate {
                let eligible = change.is_some() || {
                    let world = &index.state.world[index.world(site)];
                    world.frame == self.frame
                        && self.preparation.as_ref().map_or_else(
                            || {
                                self.pattern[position].iter().all(|term| {
                                    world.particle.iter().any(|token| term.matches(token))
                                })
                            },
                            |context| context.matches(position, index, site),
                        )
                };
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
                context.select(position, index, member.site, self.shared.as_deref())
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
        self.retained -= self.subscription.len() * 2;
        self.subscription = Box::new([]);
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
        self.subscription.len() * 2
            + self.pattern.iter().map(Vec::len).sum::<usize>()
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
