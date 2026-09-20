use super::trace::Selection;
use crate::slot::Slot;

#[derive(Eq, Hash, PartialEq)]
pub(super) struct Dependency {
    site: usize,
    binding: Vec<Selection>,
}

impl Dependency {
    pub fn new(site: usize, binding: &[Slot]) -> Self {
        Self {
            site,
            binding: binding
                .iter()
                .map(|slot| Selection {
                    site: slot.world,
                    token: slot.token.clone(),
                })
                .collect(),
        }
    }

    pub fn site(&self) -> impl Iterator<Item = usize> + '_ {
        std::iter::once(self.site).chain(
            self.binding
                .iter()
                .map(|slot| slot.site)
                .filter(|&site| site != self.site),
        )
    }

    pub fn retained(&self) -> usize {
        2 + self
            .binding
            .iter()
            .map(|slot| slot.token.len() + 1)
            .sum::<usize>()
            + self.site().count() * 2
    }
}
