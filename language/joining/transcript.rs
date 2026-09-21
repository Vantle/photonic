use super::trace::Record;
use imbl::Vector;
use std::sync::Arc;

#[derive(Clone, Default)]
pub(super) enum Transcript {
    #[default]
    Empty,
    Single(Record),
    Flat(Vec<Record>),
    Tree(Box<Chunk>),
}

#[derive(Clone, Default)]
pub(super) struct Chunk {
    prefix: Vector<Arc<Vec<Record>>>,
    tail: Vec<Record>,
}

impl Transcript {
    pub fn len(&self) -> usize {
        match self {
            Self::Empty => 0,
            Self::Single(_) => 1,
            Self::Flat(record) => record.len(),
            Self::Tree(record) => record.prefix.len() * 32 + record.tail.len(),
        }
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, position: usize) -> Option<&Record> {
        match self {
            Self::Empty => None,
            Self::Single(record) => (position == 0).then_some(record),
            Self::Flat(record) => record.get(position),
            Self::Tree(record) => {
                let boundary = record.prefix.len() * 32;
                if position >= boundary {
                    return record.tail.get(position - boundary);
                }
                record.prefix.get(position / 32)?.get(position % 32)
            }
        }
    }

    pub fn first(&self) -> Option<&Record> {
        self.get(0)
    }

    pub fn last(&self) -> Option<&Record> {
        match self {
            Self::Empty => None,
            Self::Single(record) => Some(record),
            Self::Flat(record) => record.last(),
            Self::Tree(record) => record.tail.last(),
        }
    }

    pub fn last_mut(&mut self) -> Option<&mut Record> {
        match self {
            Self::Empty => None,
            Self::Single(record) => Some(record),
            Self::Flat(record) => record.last_mut(),
            Self::Tree(record) => record.tail.last_mut(),
        }
    }

    pub fn push(&mut self, value: Record) {
        if matches!(self, Self::Empty) {
            *self = Self::Single(value);
            return;
        }
        if matches!(self, Self::Single(_)) {
            let Self::Single(record) = std::mem::take(self) else {
                unreachable!()
            };
            *self = Self::Flat(vec![record, value]);
            return;
        }
        if matches!(self, Self::Flat(record) if record.len() == 32) {
            let Self::Flat(record) = std::mem::take(self) else {
                unreachable!()
            };
            let mut chunk = Chunk::default();
            chunk.prefix.push_back(Arc::new(record));
            chunk.tail = Vec::with_capacity(32);
            *self = Self::Tree(Box::new(chunk));
        }
        match self {
            Self::Empty | Self::Single(_) => unreachable!(),
            Self::Flat(record) => record.push(value),
            Self::Tree(record) => {
                if record.tail.len() == 32 {
                    let tail = std::mem::replace(&mut record.tail, Vec::with_capacity(32));
                    record.prefix.push_back(Arc::new(tail));
                }
                record.tail.push(value);
            }
        }
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &Record> {
        (0..self.len()).map(|position| self.get(position).unwrap())
    }
}

impl Extend<Record> for Transcript {
    fn extend<Source: IntoIterator<Item = Record>>(&mut self, source: Source) {
        let mut source = source.into_iter();
        loop {
            let tail = match self {
                Self::Empty | Self::Single(_) => {
                    let Some(record) = source.next() else {
                        return;
                    };
                    self.push(record);
                    continue;
                }
                Self::Flat(record) => record,
                Self::Tree(record) => &mut record.tail,
            };
            tail.extend(source.by_ref().take(32 - tail.len()));
            let Some(record) = source.next() else {
                return;
            };
            self.push(record);
        }
    }
}

impl std::ops::Index<usize> for Transcript {
    type Output = Record;

    fn index(&self, index: usize) -> &Record {
        self.get(index).unwrap()
    }
}
