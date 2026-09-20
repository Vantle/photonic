use crate::index::Index;
use crate::particle::Match;
use crate::slot::Slot;
use crate::term::Term;
use smallvec::{SmallVec, smallvec};
use std::sync::Arc;
use std::task::Poll;

struct Member {
    site: usize,
    particle: Option<Match>,
}

pub(crate) struct Join {
    pattern: Arc<Vec<Vec<Term>>>,
    domain: SmallVec<[Vec<Member>; 2]>,
    order: SmallVec<[usize; 2]>,
    cursor: SmallVec<[usize; 2]>,
    scan: SmallVec<[bool; 2]>,
    binding: SmallVec<[Slot; 2]>,
    depth: usize,
    complete: bool,
    viable: bool,
    retained: usize,
}

impl Join {
    pub fn new(pattern: Arc<Vec<Vec<Term>>>, index: &Index, frame: usize) -> Self {
        let domain = pattern
            .iter()
            .map(|pattern| {
                index
                    .candidate(pattern, frame)
                    .into_iter()
                    .map(|world| Member {
                        site: index.site(world),
                        particle: None,
                    })
                    .collect()
            })
            .collect();
        let width = pattern.len();
        let mut join = Self {
            pattern,
            domain,
            order: (0..width).collect(),
            cursor: smallvec![0; width],
            scan: smallvec![false; width],
            binding: SmallVec::with_capacity(width),
            depth: 0,
            complete: false,
            viable: false,
            retained: 0,
        };
        join.order
            .sort_by_key(|&position| join.domain[position].len());
        join.viable = join.feasible();
        join.reset();
        join.retained = join.size();
        join
    }

    pub fn advance(&mut self, index: &Index, frame: usize) {
        for (position, domain) in self.domain.iter_mut().enumerate() {
            domain.retain(|member| !index.removal.contains(&member.site));
            for &site in &index.insertion {
                let world = &index.state.world[index.world(site)];
                if world.frame != frame {
                    continue;
                }
                if self.pattern[position]
                    .iter()
                    .all(|term| world.particle.iter().any(|token| term.matches(token)))
                {
                    domain.push(Member {
                        site,
                        particle: None,
                    });
                }
            }
        }
        self.order
            .sort_by_key(|&position| self.domain[position].len());
        self.viable = self.feasible();
        self.reset();
        self.retained = self.size();
    }

    pub fn reset(&mut self) {
        self.cursor.fill(0);
        self.scan.fill(false);
        self.retained -= self
            .binding
            .iter()
            .map(|slot| slot.token.len() + 1)
            .sum::<usize>();
        self.binding.clear();
        self.depth = 0;
        self.complete = self.domain.iter().any(Vec::is_empty);
    }

    pub fn viable(&self) -> bool {
        self.viable
    }

    fn feasible(&self) -> bool {
        if self.domain.iter().any(Vec::is_empty) {
            return false;
        }
        if self.domain.len() <= 1 {
            return true;
        }
        let mut selected = Vec::with_capacity(self.domain.len());
        for &position in &self.order {
            if let Some(member) = self.domain[position]
                .iter()
                .find(|member| !selected.contains(&member.site))
            {
                selected.push(member.site);
            } else {
                break;
            }
        }
        if selected.len() == self.domain.len() {
            return true;
        }
        let domain = self
            .domain
            .iter()
            .map(|domain| domain.iter().map(|member| member.site).collect())
            .collect::<Vec<Vec<_>>>();
        domain.iter().all(|domain| !domain.is_empty()) && crate::assignment::feasible(&domain)
    }

    pub fn step(&mut self, index: &Index) -> Poll<Option<Vec<Slot>>> {
        if self.complete {
            return Poll::Ready(None);
        }
        if self.order.is_empty() {
            self.complete = true;
            return Poll::Ready(Some(Vec::new()));
        }
        let position = self.order[self.depth];
        if !self.scan[self.depth] {
            let Some(member) = self.domain[position].get_mut(self.cursor[self.depth]) else {
                self.cursor[self.depth] = 0;
                if self.depth == 0 {
                    self.complete = true;
                    return Poll::Ready(None);
                }
                self.depth -= 1;
                let slot = self.binding.pop().unwrap();
                self.retained -= slot.token.len() + 1;
                return Poll::Pending;
            };
            if self.binding.iter().any(|slot| {
                slot.world == member.site
                    || (self.pattern[slot.position] == self.pattern[position]
                        && index.world(slot.world) >= index.world(member.site))
            }) {
                self.cursor[self.depth] += 1;
                return Poll::Pending;
            }
            if member.particle.is_none() {
                let particle = Match::new(
                    &self.pattern[position],
                    &index.state.world[index.world(member.site)].particle,
                );
                self.retained += particle.retained();
                member.particle = Some(particle);
            }
            member.particle.as_mut().unwrap().reset();
            self.scan[self.depth] = true;
        }
        match self.domain[position][self.cursor[self.depth]]
            .particle
            .as_mut()
            .unwrap()
            .step()
        {
            Poll::Ready(Some(token)) => {
                let slot = Slot {
                    world: self.domain[position][self.cursor[self.depth]].site,
                    token,
                    position,
                };
                if self.depth + 1 == self.order.len() {
                    let mut value = self.binding.clone();
                    value.push(slot);
                    for slot in &mut value {
                        slot.world = index.world(slot.world);
                    }
                    value.sort_by_key(|slot| slot.position);
                    return Poll::Ready(Some(value.into_vec()));
                }
                self.retained += slot.token.len() + 1;
                self.binding.push(slot);
                self.depth += 1;
            }
            Poll::Ready(None) => {
                self.scan[self.depth] = false;
                self.cursor[self.depth] += 1;
            }
            Poll::Pending => {}
        }
        Poll::Pending
    }

    pub fn retained(&self) -> usize {
        self.retained
    }

    fn size(&self) -> usize {
        self.pattern.iter().map(Vec::len).sum::<usize>()
            + self
                .domain
                .iter()
                .map(|domain| {
                    domain
                        .iter()
                        .map(|member| member.particle.as_ref().map_or(0, Match::retained) + 1)
                        .sum::<usize>()
                        + 1
                })
                .sum::<usize>()
            + self.order.len()
            + self.cursor.len()
            + self.scan.len()
            + self
                .binding
                .iter()
                .map(|slot| slot.token.len() + 1)
                .sum::<usize>()
    }
}
