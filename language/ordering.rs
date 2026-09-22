use std::collections::BTreeMap;

struct Group {
    order: Vec<usize>,
    member: Vec<Vec<usize>>,
}

impl Group {
    fn new(mut member: Vec<Vec<usize>>) -> Self {
        for value in &mut member {
            value.sort();
        }
        let mut order = member
            .iter()
            .enumerate()
            .flat_map(|(class, value)| value.iter().map(move |index| (*index, class)))
            .collect::<Vec<_>>();
        order.sort();
        if member.iter().all(|value| value.len() == 1) {
            return Self {
                order: order.into_iter().map(|(index, _)| index).collect(),
                member: Vec::new(),
            };
        }
        Self {
            order: order.into_iter().map(|(_, class)| class).collect(),
            member,
        }
    }

    fn smallest(&self, used: &[usize], after: Option<usize>) -> Option<usize> {
        self.member
            .iter()
            .enumerate()
            .filter_map(|(class, member)| {
                member.get(used[class]).copied().map(|index| (index, class))
            })
            .filter(|(index, _)| after.is_none_or(|after| *index > after))
            .min()
            .map(|(_, class)| class)
    }

    fn fill(&mut self, start: usize, used: &mut [usize]) {
        for index in start..self.order.len() {
            let class = self.smallest(used, None).unwrap();
            self.order[index] = class;
            used[class] += 1;
        }
    }

    fn advance(&mut self) -> bool {
        if self.member.is_empty() {
            let Some(pivot) = (1..self.order.len())
                .rev()
                .find(|&index| self.order[index - 1] < self.order[index])
            else {
                self.order.reverse();
                return false;
            };
            let next = (pivot..self.order.len())
                .rev()
                .find(|&index| self.order[pivot - 1] < self.order[index])
                .unwrap();
            self.order.swap(pivot - 1, next);
            self.order[pivot..].reverse();
            return true;
        }
        let mut used = self.member.iter().map(Vec::len).collect::<Vec<_>>();
        for index in (0..self.order.len()).rev() {
            let class = self.order[index];
            used[class] -= 1;
            let current = self.member[class][used[class]];
            if let Some(class) = self.smallest(&used, Some(current)) {
                self.order[index] = class;
                used[class] += 1;
                self.fill(index + 1, &mut used);
                return true;
            }
        }
        self.fill(0, &mut used);
        false
    }

    fn append(&self, value: &mut Vec<usize>) {
        if self.member.is_empty() {
            value.extend(&self.order);
            return;
        }
        let mut used = vec![0; self.member.len()];
        for &class in &self.order {
            value.push(self.member[class][used[class]]);
            used[class] += 1;
        }
    }
}

pub struct Ordering {
    group: Vec<Group>,
    fresh: bool,
    complete: bool,
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
        Self {
            group: group
                .into_values()
                .map(|mut order| {
                    order.sort();
                    Group {
                        order,
                        member: Vec::new(),
                    }
                })
                .collect(),
            fresh: true,
            complete: false,
        }
    }

    pub(crate) fn refine<Key: Ord>(mut self, key: impl Fn(usize) -> Key) -> Self {
        let mut group = Vec::new();
        for current in self.group {
            if current.order.len() <= 1 {
                group.push(current);
                continue;
            }
            group.extend(Self::new(current.order, &key).group);
        }
        self.group = group;
        self
    }

    pub(crate) fn quotient(mut self, representative: impl FnOnce() -> Vec<usize>) -> Self {
        if self.group.iter().all(|group| group.order.len() <= 1) {
            return self;
        }
        let representative = representative();
        self.group = self
            .group
            .into_iter()
            .map(|group| {
                let mut member = BTreeMap::<usize, Vec<usize>>::new();
                for index in group.order {
                    member.entry(representative[index]).or_default().push(index);
                }
                Group::new(member.into_values().collect())
            })
            .collect();
        self
    }
}

impl Iterator for Ordering {
    type Item = Vec<usize>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.complete {
            return None;
        }
        if !self.fresh && !self.group.iter_mut().rev().any(Group::advance) {
            self.complete = true;
            return None;
        }
        self.fresh = false;
        let mut value = Vec::new();
        for group in &self.group {
            group.append(&mut value);
        }
        Some(value)
    }
}

#[cfg(test)]
#[path = "test/ordering.rs"]
mod test;
