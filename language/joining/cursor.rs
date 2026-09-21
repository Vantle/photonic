use super::space::Space;
use crate::index::Index;
use crate::slot::Slot;
use smallvec::{SmallVec, smallvec};
use std::task::Poll;

pub(super) struct Cursor {
    cursor: SmallVec<[usize; 2]>,
    scan: SmallVec<[bool; 2]>,
    binding: SmallVec<[Slot; 2]>,
    occupied: u64,
    depth: usize,
    complete: bool,
    retained: usize,
}

impl Cursor {
    pub fn new(width: usize) -> Self {
        Self {
            cursor: smallvec![0; width],
            scan: smallvec![false; width],
            binding: SmallVec::with_capacity(width),
            occupied: 0,
            depth: 0,
            complete: false,
            retained: width * 2,
        }
    }

    pub fn reset(&mut self) {
        self.cursor.fill(0);
        self.scan.fill(false);
        self.binding.clear();
        self.occupied = 0;
        self.depth = 0;
        self.complete = false;
        self.retained = self.cursor.len() + self.scan.len();
    }

    pub fn boundary(&self, depth: usize) -> Option<usize> {
        (!self.complete && self.depth == depth && !self.scan[depth]).then(|| self.cursor[depth])
    }

    pub fn seek(&mut self, depth: usize, position: usize) {
        self.cursor[depth] = position;
        self.cursor[depth + 1..].fill(0);
        self.scan[depth..].fill(false);
    }

    pub fn binding(&self) -> &[Slot] {
        &self.binding
    }

    pub fn seed(&mut self, binding: Vec<Slot>) {
        self.reset();
        self.retained += binding
            .iter()
            .map(|slot| slot.token.len() + 1)
            .sum::<usize>();
        self.binding = SmallVec::from_vec(binding);
        self.occupied = self
            .binding
            .iter()
            .filter(|slot| slot.world < 64)
            .fold(0, |occupied, slot| occupied | (1 << slot.world));
    }

    #[cfg(test)]
    pub fn size(&self) -> usize {
        self.cursor.len()
            + self.scan.len()
            + self
                .binding
                .iter()
                .map(|slot| slot.token.len() + 1)
                .sum::<usize>()
    }

    pub fn retained(&self) -> usize {
        self.retained
    }

    #[inline]
    pub fn step(
        &mut self,
        space: &mut Space,
        order: &[usize],
        index: &Index,
    ) -> Poll<Option<Vec<Slot>>> {
        if self.complete {
            return Poll::Ready(None);
        }
        if order.is_empty() {
            self.complete = true;
            return Poll::Ready(Some(Vec::new()));
        }
        let position = order[self.depth];
        if !self.scan[self.depth] {
            let Some(member) = space.domain[position].get_mut(self.cursor[self.depth]) else {
                self.cursor[self.depth] = 0;
                if self.depth == 0 {
                    self.complete = true;
                    return Poll::Ready(None);
                }
                self.depth -= 1;
                let slot = self.binding.pop().unwrap();
                if slot.world < 64 {
                    self.occupied &= !(1 << slot.world);
                }
                self.retained -= slot.token.len() + 1;
                return Poll::Pending;
            };
            if member.rejected {
                self.cursor[self.depth] += 1;
                return Poll::Pending;
            }
            let blocked = self
                .binding
                .iter()
                .rev()
                .find(|slot| space.group[slot.position] == space.group[position])
                .is_some_and(|slot| !index.precedes(slot.world, member.site))
                || if member.site < 64 {
                    self.occupied & (1 << member.site) != 0
                } else {
                    self.binding.iter().any(|slot| slot.world == member.site)
                };
            #[cfg(test)]
            assert_eq!(
                blocked,
                self.binding.iter().any(|slot| {
                    slot.world == member.site
                        || (space.group[slot.position] == space.group[position]
                            && !index.precedes(slot.world, member.site))
                })
            );
            if blocked {
                self.cursor[self.depth] += 1;
                return Poll::Pending;
            }
            space
                .prepare(position, self.cursor[self.depth], index)
                .reset();
            self.scan[self.depth] = true;
        }
        let particle = space.domain[position][self.cursor[self.depth]]
            .particle
            .as_mut()
            .unwrap();
        let result = if space.store.is_some() {
            let previous = particle.cached();
            let result = particle.step(4096 - space.cached);
            space.cached = space.cached - previous + particle.cached();
            space.retained = space.retained - previous + particle.cached();
            result
        } else {
            particle.step(0)
        };
        match result {
            Poll::Ready(Some(token)) => {
                let slot = Slot {
                    world: space.domain[position][self.cursor[self.depth]].site,
                    token,
                    position,
                };
                if self.depth + 1 == order.len() {
                    let mut value = self.binding.clone();
                    value.push(slot);
                    return Poll::Ready(Some(value.into_vec()));
                }
                self.retained += slot.token.len() + 1;
                if slot.world < 64 {
                    self.occupied |= 1 << slot.world;
                }
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
}
