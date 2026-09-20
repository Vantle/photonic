use super::space::Space;
use crate::index::Index;
use crate::slot::Slot;
use std::task::Poll;

pub(super) trait Prefix {
    fn reset(&mut self, index: &Index);
    fn step(
        &mut self,
        space: &mut Space,
        order: &[usize],
        index: &Index,
    ) -> Poll<Option<Vec<Slot>>>;
    fn evict(&mut self);
    fn retained(&self) -> usize;
    fn cached(&self) -> usize;
    #[cfg(test)]
    fn size(&self) -> usize;
}
