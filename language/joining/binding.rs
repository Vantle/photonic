use super::slot::Slot;
use std::sync::Arc;

#[derive(Clone, Eq, Hash, PartialEq)]
pub(super) struct Selection {
    pub site: usize,
    pub token: Vec<usize>,
}

#[derive(Clone)]
enum Storage {
    Leaf(Arc<[Selection]>),
    Branch(Arc<Branch>),
}

struct Branch {
    prefix: Arc<[Selection]>,
    suffix: Binding,
    width: usize,
    retained: usize,
}

#[derive(Clone)]
pub(super) struct Binding(Storage);

impl Binding {
    pub fn new(value: &[Slot]) -> Self {
        Self(Storage::Leaf(
            value
                .iter()
                .map(|slot| Selection {
                    site: slot.site,
                    token: slot.token.clone(),
                })
                .collect(),
        ))
    }

    pub fn len(&self) -> usize {
        match &self.0 {
            Storage::Leaf(value) => value.len(),
            Storage::Branch(value) => value.width,
        }
    }

    pub fn retained(&self) -> usize {
        match &self.0 {
            Storage::Leaf(value) => {
                1 + value
                    .iter()
                    .map(|selection| selection.token.len() + 1)
                    .sum::<usize>()
            }
            Storage::Branch(value) => value.retained,
        }
    }

    pub fn prepend(&self, prefix: Arc<[Selection]>) -> Self {
        if prefix.is_empty() {
            return self.clone();
        }
        let retained = self.retained()
            + 1
            + prefix
                .iter()
                .map(|selection| selection.token.len() + 1)
                .sum::<usize>();
        Self(Storage::Branch(Arc::new(Branch {
            width: prefix.len() + self.len(),
            retained,
            prefix,
            suffix: self.clone(),
        })))
    }

    pub fn append(&self, output: &mut Vec<Slot>, order: &[usize]) {
        let mut binding = self;
        loop {
            let (value, next) = match &binding.0 {
                Storage::Leaf(value) => (value.as_ref(), None),
                Storage::Branch(value) => (value.prefix.as_ref(), Some(&value.suffix)),
            };
            for selection in value {
                output.push(Slot {
                    site: selection.site,
                    position: order[output.len()],
                    token: selection.token.clone(),
                });
            }
            let Some(next) = next else {
                return;
            };
            binding = next;
        }
    }

    #[cfg(test)]
    pub fn size(&self) -> usize {
        match &self.0 {
            Storage::Leaf(value) => {
                1 + value
                    .iter()
                    .map(|selection| selection.token.len() + 1)
                    .sum::<usize>()
            }
            Storage::Branch(value) => {
                1 + value
                    .prefix
                    .iter()
                    .map(|selection| selection.token.len() + 1)
                    .sum::<usize>()
                    + value.suffix.size()
            }
        }
    }
}
