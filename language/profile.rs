#[derive(Clone, Copy)]
#[cfg_attr(
    feature = "measurement",
    derive(serde::Serialize),
    serde(rename_all = "lowercase")
)]
pub(crate) enum Phase {
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
    Discovery,
    Planning,
    Imaging,
    Carriage,
    Insertion,
    Identification,
    Firing,
    Creation,
    Marking,
    Support,
    Deduction,
}

#[cfg(not(feature = "measurement"))]
pub(crate) struct Scope;

#[cfg(not(feature = "measurement"))]
impl Scope {
    #[inline]
    pub fn new(_: Phase) -> Self {
        Self
    }
}

#[cfg(feature = "measurement")]
pub(crate) struct Scope {
    phase: Phase,
    start: std::time::Instant,
}

#[cfg(feature = "measurement")]
impl Scope {
    pub fn new(phase: Phase) -> Self {
        Self {
            phase,
            start: std::time::Instant::now(),
        }
    }
}

#[cfg(feature = "measurement")]
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

#[cfg(feature = "measurement")]
const PHASE: [Phase; 37] = [
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
    Phase::Discovery,
    Phase::Planning,
    Phase::Imaging,
    Phase::Carriage,
    Phase::Insertion,
    Phase::Identification,
    Phase::Firing,
    Phase::Creation,
    Phase::Marking,
    Phase::Support,
    Phase::Deduction,
];

#[cfg(feature = "measurement")]
const _: () = {
    let mut position = 0;
    while position < PHASE.len() {
        assert!(PHASE[position] as usize == position);
        position += 1;
    }
};

#[cfg(feature = "measurement")]
thread_local! {
    static RECORD: std::cell::RefCell<[[u64; 2]; PHASE.len()]> =
        const { std::cell::RefCell::new([[0; 2]; PHASE.len()]) };
}

#[cfg(feature = "measurement")]
#[derive(serde::Serialize)]
pub struct Measurement {
    phase: Phase,
    count: u64,
    duration: f64,
}

#[cfg(feature = "measurement")]
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
