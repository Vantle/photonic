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
    subscription: SmallVec<[(usize, Arc<crate::candidate::Node>); 2]>,
    pub retained: usize,
}

impl Space {
    pub fn dependency(
        &self,
        site: usize,
        binding: &[super::slot::Slot],
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
                        .any(|&position| self.query.matches(position, index, slot.site))
                })
                .map(|slot| (slot.position, slot.site)),
        )
    }

    #[cfg(any(test, feature = "measurement"))]
    pub fn new(
        query: Query,
        index: &Index,
        frame: usize,
        store: Option<&Arc<super::Store>>,
    ) -> Self {
        Self::construct(query, index, frame, store, false).unwrap()
    }

    pub fn admit(
        query: Query,
        index: &Index,
        frame: usize,
        store: Option<&Arc<super::Store>>,
    ) -> Option<Self> {
        Self::construct(query, index, frame, store, true)
    }

    fn construct(
        query: Query,
        index: &Index,
        frame: usize,
        store: Option<&Arc<super::Store>>,
        admission: bool,
    ) -> Option<Self> {
        if admission && !query.possible(index, frame) {
            return None;
        }
        let mut subscription = SmallVec::<[(usize, Arc<crate::candidate::Node>); 2]>::new();
        let mut domain = SmallVec::<[Vec<Member>; 2]>::new();
        for position in 0..query.count() {
            let shared = store
                .filter(|_| index.state.world.len() > 32 && query.width(position) > 1)
                .map(|store| &store.domain);
            let selected = query.candidate(position, index, frame, shared);
            if admission && selected.site.is_empty() {
                return None;
            }
            if let Some(node) = selected.node {
                subscription.push((position, node));
            }
            domain.push(
                selected
                    .site
                    .into_iter()
                    .map(|site| Member {
                        site,
                        particle: None,
                        rejected: false,
                    })
                    .collect(),
            );
        }
        let retained = subscription.len() * 2
            + query.retained()
            + domain.iter().map(|member| member.len() + 1).sum::<usize>();
        let shared = store
            .filter(|_| {
                (0..query.count()).any(|position| crate::particle::wide(query.width(position)))
            })
            .map(|store| store.preparation.clone());
        let store = store
            .filter(|_| {
                query.count() > 1
                    && (0..query.count())
                        .any(|position| crate::particle::wide(query.width(position)))
            })
            .cloned();
        Some(Self {
            frame,
            store,
            shared,
            cached: 0,
            query,
            domain,
            subscription,
            retained,
        })
    }

    pub fn update(&mut self, index: &Index) -> SmallVec<[usize; 2]> {
        let mut changed = SmallVec::new();
        for (position, domain) in self.domain.iter_mut().enumerate() {
            let change = self
                .subscription
                .iter()
                .find(|(input, _)| *input == position)
                .filter(|(_, node)| Arc::strong_count(node) > 2)
                .map(|(_, node)| node.change(index));
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
                if !index.removed(member.site) {
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
                .map_or(index.delta().insertion.as_slice(), |change| {
                    change.insertion.site.as_slice()
                });
            for &site in candidate {
                let eligible = change.is_some() || {
                    index.location(site).frame(&index.state) == self.frame
                        && self.query.matches(position, index, site)
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
                .filter(|_| crate::particle::wide(self.query.width(position)))
                .map(|store| store.budget().clone());
            member.rejected = !particle.viable();
            member.particle = Some(crate::factor::Cursor::new(particle, budget));
        }
        member.particle.as_mut().unwrap()
    }

    pub fn evict(&mut self) {
        self.retained -= self.subscription.len() * 2;
        self.subscription = SmallVec::new();
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
