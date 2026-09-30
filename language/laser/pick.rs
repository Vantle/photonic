use smallvec::SmallVec;

// The picks of a rule joining several coherences, one option for each input, the first input
// moving fastest. Picks that differ only in which of several inputs with the same options took
// which option bind the same part, so an input picks no earlier option than the next input with
// its options, and each such pick comes once, where the first of its reorderings comes.
pub(super) struct Pick {
    width: SmallVec<[usize; 4]>,
    next: SmallVec<[Option<usize>; 4]>,
}

// The multisets of a size drawn from so many options, or the most a count holds.
fn multiset(option: usize, size: usize) -> usize {
    let mut count = 1u128;
    for index in 0..size as u128 {
        count = count.saturating_mul(option as u128 + index) / (index + 1);
    }
    usize::try_from(count).unwrap_or(usize::MAX)
}

impl Pick {
    // The picks of inputs with these numbers of options, where same tells whether two inputs have
    // the same options; an input with one option picks it whatever the others pick.
    pub fn new(width: SmallVec<[usize; 4]>, same: impl Fn(usize, usize) -> bool) -> Self {
        let next = (0..width.len())
            .map(|input| {
                (width[input] > 1)
                    .then(|| {
                        (input + 1..width.len())
                            .find(|&later| width[later] == width[input] && same(input, later))
                    })
                    .flatten()
            })
            .collect();
        Self { width, next }
    }

    // How many picks there are: for each set of inputs with the same options, the multisets of its
    // size drawn from its options.
    pub fn count(&self) -> usize {
        (0..self.width.len())
            .filter(|&input| self.width[input] > 1 && !self.next.contains(&Some(input)))
            .map(|input| {
                let size = std::iter::successors(Some(input), |&input| self.next[input]).count();
                multiset(self.width[input], size)
            })
            .fold(1, usize::saturating_mul)
    }

    pub fn first(&self) -> SmallVec<[usize; 4]> {
        SmallVec::from_elem(0, self.width.len())
    }

    // Moves to the next pick: the first input below its last option takes the next one, and every
    // input before it returns to the least option it may take. False once every pick is made.
    pub fn advance(&self, digit: &mut [usize]) -> bool {
        let Some(input) = (0..digit.len()).find(|&input| digit[input] + 1 < self.width[input])
        else {
            return false;
        };
        digit[input] += 1;
        for earlier in (0..input).rev() {
            digit[earlier] = self.next[earlier].map_or(0, |next| digit[next]);
        }
        true
    }
}

#[cfg(test)]
#[path = "../test/pick.rs"]
mod test;
