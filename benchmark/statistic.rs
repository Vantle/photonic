use serde::Serialize;

// A sample is built from its first measurement, so its median and extremes always exist.
#[derive(Clone, Debug)]
pub struct Sample {
    second: Vec<f64>,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Spread {
    pub count: usize,
    pub minimum: f64,
    pub median: f64,
    pub maximum: f64,
}

impl Sample {
    pub fn new(second: f64) -> Self {
        Self {
            second: vec![second],
        }
    }

    pub fn push(&mut self, second: f64) {
        self.second.push(second);
    }

    pub fn spread(&self) -> Spread {
        let mut sorted = self.second.clone();
        sorted.sort_by(f64::total_cmp);
        let middle = sorted.len() / 2;
        let median = if sorted.len().is_multiple_of(2) {
            f64::midpoint(sorted[middle - 1], sorted[middle])
        } else {
            sorted[middle]
        };
        Spread {
            count: sorted.len(),
            minimum: sorted[0],
            median,
            maximum: sorted[sorted.len() - 1],
        }
    }
}

#[cfg(test)]
#[path = "test/statistic.rs"]
mod test;
