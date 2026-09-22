#[derive(Clone, Eq, PartialEq)]
pub(super) struct Selection {
    pub word: Vec<u64>,
    pub count: usize,
}

impl Selection {
    pub fn new(count: usize) -> Self {
        let mut word = vec![u64::MAX; count.div_ceil(64)];
        if !count.is_multiple_of(64) {
            *word.last_mut().unwrap() = (1 << (count % 64)) - 1;
        }
        Self { word, count }
    }

    pub fn remove(&mut self, position: usize) {
        let word = &mut self.word[position / 64];
        let bit = 1 << (position % 64);
        if *word & bit != 0 {
            *word &= !bit;
            self.count -= 1;
        }
    }
}
