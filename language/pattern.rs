use crate::program::Symbol;
use smallvec::SmallVec;
use std::sync::Arc;

pub(crate) struct Group<Value> {
    pub value: Value,
    pub position: Arc<Vec<usize>>,
}

pub(crate) struct Pattern<Value = Symbol> {
    pub group: SmallVec<[Group<Value>; 2]>,
    pub width: usize,
}

impl<Value: Clone + PartialEq> Pattern<Value> {
    pub fn new(value: &[Value]) -> Self {
        let mut group: SmallVec<[Group<Value>; 2]> = SmallVec::new();
        for (position, value) in value.iter().enumerate() {
            if let Some(group) = group.iter_mut().find(|group| group.value == *value) {
                Arc::make_mut(&mut group.position).push(position);
            } else {
                group.push(Group {
                    value: value.clone(),
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
