use serde::Serialize;
use std::cell::RefCell;
use std::time::Instant;

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Matching,
    Dispatch,
    Index,
    Rewrite,
    Fingerprint,
    Structure,
    Canonicalization,
    Composition,
    Application,
    Subscription,
    Refresh,
    Restart,
    Request,
    Preparation,
    Availability,
    Context,
    Removal,
    Replacement,
    Admission,
    Normalization,
    Rendering,
    Incidence,
    Refinement,
    Renaming,
    Dependency,
    Color,
}

const PHASE: [Phase; 26] = [
    Phase::Matching,
    Phase::Dispatch,
    Phase::Index,
    Phase::Rewrite,
    Phase::Fingerprint,
    Phase::Structure,
    Phase::Canonicalization,
    Phase::Composition,
    Phase::Application,
    Phase::Subscription,
    Phase::Refresh,
    Phase::Restart,
    Phase::Request,
    Phase::Preparation,
    Phase::Availability,
    Phase::Context,
    Phase::Removal,
    Phase::Replacement,
    Phase::Admission,
    Phase::Normalization,
    Phase::Rendering,
    Phase::Incidence,
    Phase::Refinement,
    Phase::Renaming,
    Phase::Dependency,
    Phase::Color,
];

thread_local! {
    static RECORD: RefCell<[[u64; 2]; PHASE.len()]> = const { RefCell::new([[0; 2]; PHASE.len()]) };
}

pub(crate) struct Scope {
    phase: Phase,
    start: Instant,
}

impl Scope {
    pub fn new(phase: Phase) -> Self {
        Self {
            phase,
            start: Instant::now(),
        }
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed().as_nanos().min(u64::MAX as u128) as u64;
        RECORD.with_borrow_mut(|record| {
            let record = &mut record[self.phase as usize];
            record[0] = record[0].saturating_add(1);
            record[1] = record[1].saturating_add(elapsed);
        });
    }
}

#[derive(Serialize)]
pub struct Measurement {
    phase: Phase,
    count: u64,
    duration: f64,
}

pub fn take() -> Vec<Measurement> {
    RECORD.with_borrow_mut(|record| {
        let value = PHASE
            .into_iter()
            .zip(record.iter())
            .map(|(phase, &[count, duration])| Measurement {
                phase,
                count,
                duration: duration as f64 / 1_000_000_000.0,
            })
            .collect();
        *record = [[0; 2]; PHASE.len()];
        value
    })
}
