use crate::program::Symbol;
use smallvec::SmallVec;
use std::sync::Arc;

pub(crate) struct Group {
    pub value: Symbol,
    pub position: Arc<Vec<usize>>,
}

pub(crate) struct Pattern {
    pub group: SmallVec<[Group; 2]>,
    pub width: usize,
}

impl Pattern {
    pub fn new(value: &[Symbol]) -> Self {
        let mut group: SmallVec<[Group; 2]> = SmallVec::new();
        for (position, &value) in value.iter().enumerate() {
            if let Some(group) = group.iter_mut().find(|group| group.value == value) {
                Arc::make_mut(&mut group.position).push(position);
            } else {
                group.push(Group {
                    value,
                    position: Arc::new(vec![position]),
                });
            }
        }
        Self {
            group,
            width: value.len(),
        }
    }
}
