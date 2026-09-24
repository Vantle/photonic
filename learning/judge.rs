pub const MARGIN: f64 = 0.02;
pub const EXPLORE: f64 = 0.05;
pub const PROBE: f64 = 0.1;
const MEMORY: f64 = 0.999;
const MINIMUM: u64 = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Infer {
    Never,
    Probe,
    Trusted,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Recall {
    pub worthy: f64,
    pub lost: f64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Judge {
    pub total: f64,
    pub count: u64,
    pub skipped: u64,
    pub sample: u64,
    pub recall: Recall,
    recent: Recall,
}

impl Judge {
    pub fn record(&mut self, miss: f64) {
        self.total += miss;
        self.count += 1;
    }

    pub fn observe(&mut self, doubtful: bool, weight: f64) {
        let lost = if doubtful { weight } else { 0.0 };
        self.sample += 1;
        self.recall = Recall {
            worthy: self.recall.worthy + weight,
            lost: self.recall.lost + lost,
        };
        self.recent = Recall {
            worthy: MEMORY * self.recent.worthy + weight,
            lost: MEMORY * self.recent.lost + lost,
        };
    }

    pub fn mode(&self, bound: f64) -> Infer {
        if bound <= 0.0 {
            return Infer::Never;
        }
        if self.sample >= MINIMUM && self.recent.lost < bound * self.recent.worthy {
            return Infer::Trusted;
        }
        Infer::Probe
    }
}
