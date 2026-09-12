use std::collections::BTreeMap;

pub struct Ordering {
    group: Vec<Vec<usize>>,
    fresh: bool,
    complete: bool,
}

fn advance(value: &mut [usize]) -> bool {
    let Some(pivot) = (1..value.len())
        .rev()
        .find(|&index| value[index - 1] < value[index])
    else {
        value.reverse();
        return false;
    };
    let next = (pivot..value.len())
        .rev()
        .find(|&index| value[pivot - 1] < value[index])
        .unwrap();
    value.swap(pivot - 1, next);
    value[pivot..].reverse();
    true
}

impl Ordering {
    pub fn new<Key: Ord>(
        value: impl IntoIterator<Item = usize>,
        key: impl Fn(usize) -> Key,
    ) -> Self {
        let mut group = BTreeMap::<Key, Vec<usize>>::new();
        for index in value {
            group.entry(key(index)).or_default().push(index);
        }
        let mut group = group.into_values().collect::<Vec<_>>();
        for value in &mut group {
            value.sort();
        }
        Self {
            group,
            fresh: true,
            complete: false,
        }
    }
}

impl Iterator for Ordering {
    type Item = Vec<usize>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.complete {
            return None;
        }
        if !self.fresh && !self.group.iter_mut().rev().any(|group| advance(group)) {
            self.complete = true;
            return None;
        }
        self.fresh = false;
        Some(self.group.iter().flatten().copied().collect())
    }
}
