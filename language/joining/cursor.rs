use super::space::Space;
use crate::index::Index;
use crate::slot::Slot;
use smallvec::{SmallVec, smallvec};
use std::task::Poll;

pub(super) struct Cursor {
    cursor: SmallVec<[usize; 2]>,
    scan: SmallVec<[bool; 2]>,
    binding: SmallVec<[Slot; 2]>,
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
            depth: 0,
            complete: false,
            retained: width * 2,
        }
    }

    pub fn reset(&mut self) {
        self.cursor.fill(0);
        self.scan.fill(false);
        self.binding.clear();
        self.depth = 0;
        self.complete = false;
        self.retained = self.cursor.len() + self.scan.len();
    }

    pub fn seed(&mut self, binding: Vec<Slot>) {
        self.reset();
        self.retained += binding
            .iter()
            .map(|slot| slot.token.len() + 1)
            .sum::<usize>();
        self.binding = SmallVec::from_vec(binding);
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
                self.retained -= slot.token.len() + 1;
                return Poll::Pending;
            };
            if self.binding.iter().any(|slot| {
                slot.world == member.site
                    || (space.group[slot.position] == space.group[position]
                        && index.world(slot.world) >= index.world(member.site))
            }) {
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
        let result = if space.budget.is_some() {
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
