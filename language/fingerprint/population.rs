use crate::basis::Set;
use imbl::OrdMap;

#[derive(Default)]
pub(super) struct Index {
    value: crate::population::Set,
    capture: Option<OrdMap<usize, usize>>,
}

impl Index {
    pub fn advance(&self, value: &crate::population::Set) -> Self {
        if !self.value.shared(value) {
            return Self {
                value: value.clone(),
                capture: None,
            };
        }
        let Some(previous) = &self.capture else {
            let mut capture = OrdMap::new();
            for frame in value.iter().filter_map(|token| token.capture) {
                *capture.entry(frame).or_default() += 1;
            }
            return Self {
                value: value.clone(),
                capture: Some(capture),
            };
        };
        let mut capture = previous.clone();
        let (removed, inserted) = self.value.difference(value);
        for position in removed {
            let Some(frame) = self.value.at(position).capture else {
                continue;
            };
            let count = capture.get_mut(&frame).unwrap();
            *count -= 1;
            if *count == 0 {
                capture.remove(&frame);
            }
        }
        for position in inserted {
            if let Some(frame) = value.at(position).capture {
                *capture.entry(frame).or_default() += 1;
            }
        }
        Self {
            value: value.clone(),
            capture: Some(capture),
        }
    }

    pub fn reference(&self, source: impl Iterator<Item = usize>) -> Set<usize> {
        if let Some(capture) = &self.capture {
            return source.chain(capture.keys().copied()).collect();
        }
        source
            .chain(self.value.iter().filter_map(|token| token.capture))
            .collect()
    }

    pub fn retained(&self) -> usize {
        1 + self.capture.as_ref().map_or(0, OrdMap::len)
    }
}
