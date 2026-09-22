use super::query::Query;
use crate::index::Index;
use smallvec::SmallVec;
use std::sync::Arc;

pub(super) struct Member {
    pub site: usize,
    pub rejected: bool,
    pub particle: Option<crate::factor::Cursor>,
}

pub(super) struct Space {
    pub frame: usize,
    pub store: Option<Arc<super::Store>>,
    pub cached: usize,
    pub query: Query,
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
                    order[binding.len()..]
                        .iter()
                        .any(|&position| self.query.matches(position, index, slot.world))
                })
                .map(|slot| (slot.position, slot.world)),
        )
    }

    pub fn new(
        query: Query,
        index: &Index,
        frame: usize,
        store: Option<&Arc<super::Store>>,
    ) -> Self {
        let mut subscription = Vec::new();
        let domain = (0..query.count())
            .map(|position| {
                let shared = store
                    .filter(|_| index.state.world.len() > 32 && query.width(position) > 1)
                    .map(|store| &store.domain);
                let selected = query.candidate(position, index, frame, shared);
                if let Some(node) = selected.node {
                    subscription.push((position, node));
                }
                selected
                    .site
                    .into_iter()
                    .map(|site| Member {
                        site,
                        particle: None,
                        rejected: false,
                    })
                    .collect()
            })
            .collect();
        let domain: SmallVec<[Vec<Member>; 2]> = domain;
        let retained = subscription.len() * 2
            + query.retained()
            + domain.iter().map(|member| member.len() + 1).sum::<usize>();
        let shared = store
            .filter(|_| (0..query.count()).any(|position| query.width(position) >= 8))
            .map(|store| store.preparation.clone());
        let store = store
            .filter(|_| {
                query.count() > 1 && (0..query.count()).any(|position| query.width(position) >= 8)
            })
            .cloned();
        Self {
            frame,
            store,
            shared,
            cached: 0,
            query,
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
                    || !self.query.affected(position, index, self.frame),
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
                    world.frame == self.frame && self.query.matches(position, index, site)
                };
                if eligible {
                    affected = true;
                    domain.push(Member {
                        site,
                        particle: None,
                        rejected: false,
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

    #[inline]
    pub fn prepare(
        &mut self,
        position: usize,
        candidate: usize,
        index: &Index,
    ) -> &mut crate::factor::Cursor {
        let member = &mut self.domain[position][candidate];
        if member.particle.is_none() {
            let particle = self
                .query
                .select(position, index, member.site, self.shared.as_deref());
            self.retained += particle.retained();
            let budget = self
                .store
                .as_ref()
                .filter(|_| self.query.width(position) >= 8)
                .map(|store| store.budget().clone());
            member.rejected = !particle.viable();
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
            + self.query.retained()
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
