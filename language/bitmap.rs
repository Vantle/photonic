enum Storage {
    Inline(u64),
    Indexed(Box<Heap>),
}

struct Heap {
    word: Vec<u64>,
    occupied: Set,
    length: usize,
}

pub(crate) struct Set(Storage);

struct Word {
    value: u64,
    offset: usize,
}

pub(crate) struct Traversal<'set> {
    word: Word,
    remaining: Option<&'set Heap>,
    position: usize,
}

impl Set {
    pub fn new(width: usize) -> Self {
        if width <= 64 {
            return Self(Storage::Inline(0));
        }
        Self(Storage::Indexed(Box::new(Heap {
            word: vec![0; width.div_ceil(64)],
            occupied: Self::new(width.div_ceil(64)),
            length: 0,
        })))
    }

    pub fn insert(&mut self, value: usize) -> bool {
        match &mut self.0 {
            Storage::Inline(word) => {
                assert!(value < 64);
                let mask = 1 << value;
                if *word & mask != 0 {
                    return false;
                }
                *word |= mask;
            }
            Storage::Indexed(heap) => {
                let position = value / 64;
                let mask = 1 << (value % 64);
                let word = &mut heap.word[position];
                if *word & mask != 0 {
                    return false;
                }
                if *word == 0 {
                    heap.occupied.insert(position);
                }
                *word |= mask;
                heap.length += 1;
            }
        }
        true
    }

    pub fn remove(&mut self, value: &usize) -> bool {
        match &mut self.0 {
            Storage::Inline(word) => {
                if *value >= 64 {
                    return false;
                }
                let mask = 1 << value;
                if *word & mask == 0 {
                    return false;
                }
                *word &= !mask;
            }
            Storage::Indexed(heap) => {
                let position = value / 64;
                let mask = 1 << (value % 64);
                let Some(word) = heap.word.get_mut(position) else {
                    return false;
                };
                if *word & mask == 0 {
                    return false;
                }
                *word &= !mask;
                if *word == 0 {
                    heap.occupied.remove(&position);
                }
                heap.length -= 1;
            }
        }
        true
    }

    pub fn contains(&self, value: &usize) -> bool {
        match &self.0 {
            Storage::Inline(word) => *value < 64 && word & (1 << value) != 0,
            Storage::Indexed(heap) => heap
                .word
                .get(value / 64)
                .is_some_and(|word| word & (1 << (value % 64)) != 0),
        }
    }

    pub fn len(&self) -> usize {
        match &self.0 {
            Storage::Inline(word) => word.count_ones() as usize,
            Storage::Indexed(heap) => heap.length,
        }
    }

    pub fn iter(&self) -> Traversal<'_> {
        match &self.0 {
            Storage::Inline(word) => Traversal {
                word: Word {
                    value: *word,
                    offset: 0,
                },
                remaining: None,
                position: 0,
            },
            Storage::Indexed(heap) => Traversal {
                word: Word {
                    value: 0,
                    offset: 0,
                },
                remaining: Some(heap),
                position: 0,
            },
        }
    }

    fn seek(&self, start: usize) -> Option<usize> {
        match &self.0 {
            Storage::Inline(word) => {
                if start >= 64 {
                    return None;
                }
                let word = word & (u64::MAX << start);
                (word != 0).then(|| word.trailing_zeros() as usize)
            }
            Storage::Indexed(heap) => {
                let position = start / 64;
                let word = heap.word.get(position)? & (u64::MAX << (start % 64));
                if word != 0 {
                    return Some(position * 64 + word.trailing_zeros() as usize);
                }
                let position = heap.occupied.seek(position + 1)?;
                Some(position * 64 + heap.word[position].trailing_zeros() as usize)
            }
        }
    }
}

impl Iterator for Traversal<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.word.value != 0 {
                let position = self.word.value.trailing_zeros() as usize;
                self.word.value &= self.word.value - 1;
                return Some(self.word.offset + position);
            }
            let heap = self.remaining?;
            let position = heap.occupied.seek(self.position)?;
            self.position = position + 1;
            self.word = Word {
                value: heap.word[position],
                offset: position * 64,
            };
        }
    }
}

#[cfg(test)]
#[path = "test/bitmap.rs"]
mod test;
