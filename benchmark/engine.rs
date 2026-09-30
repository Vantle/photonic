use crate::fingerprint::Fingerprint;
use crate::meter::{self, Measurement};
use photonic::prism::Outcome;
use photonic::runtime::Limit;
use serde::Serialize;
use std::fmt;
use std::hint::black_box;

#[derive(Debug)]
pub enum Failure {
    Target(Outcome),
    Open,
    Encoding(serde_json::Error),
}

impl fmt::Display for Failure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Target(outcome) => write!(
                formatter,
                "the search ended {outcome:?} instead of reaching the target; raise --budget"
            ),
            Self::Open => formatter
                .write_str("the exploration did not close within the budget; raise --budget"),
            Self::Encoding(error) => write!(formatter, "the report could not be encoded: {error}"),
        }
    }
}

impl std::error::Error for Failure {}

pub trait Engine {
    type Report: Serialize;

    fn execute(&mut self, budget: usize, limit: Limit);
    fn finish(&self) -> Result<(), Failure>;
    fn report(&self) -> Self::Report;
    fn stream(&self) -> impl Serialize + '_;
    fn observe(report: &Self::Report) -> Observation;
}

#[derive(Serialize)]
pub struct Observation {
    state: usize,
    event: usize,
    work: usize,
    byte: usize,
    fingerprint: u64,
}

impl Engine for photonic::path::Search {
    type Report = photonic::path::Report;

    fn execute(&mut self, budget: usize, limit: Limit) {
        self.run(budget, limit);
    }

    fn finish(&self) -> Result<(), Failure> {
        let outcome = self.summary().outcome;
        if outcome == Outcome::Reached {
            return Ok(());
        }
        Err(Failure::Target(outcome))
    }

    fn report(&self) -> Self::Report {
        self.report()
    }

    fn stream(&self) -> impl Serialize + '_ {
        self.stream()
    }

    fn observe(report: &Self::Report) -> Observation {
        Observation {
            state: report.state.len(),
            event: report.event.len(),
            work: report.work,
            byte: 0,
            fingerprint: 0,
        }
    }
}

impl Engine for photonic::runtime::Runtime {
    type Report = photonic::snapshot::Snapshot;

    fn execute(&mut self, budget: usize, limit: Limit) {
        self.run(budget, limit);
    }

    fn finish(&self) -> Result<(), Failure> {
        if self.closed() {
            return Ok(());
        }
        Err(Failure::Open)
    }

    fn report(&self) -> Self::Report {
        self.snapshot()
    }

    fn stream(&self) -> impl Serialize + '_ {
        self.stream()
    }

    fn observe(report: &Self::Report) -> Observation {
        Observation {
            state: report.state.len(),
            event: report.event.len(),
            work: report.work,
            byte: 0,
            fingerprint: 0,
        }
    }
}

#[derive(Serialize)]
pub struct Record {
    pub initialization: Measurement,
    pub execution: Measurement,
    pub reporting: Measurement,
    pub serialization: Measurement,
    pub release: Measurement,
    duration: f64,
    observation: Observation,
    #[cfg(feature = "allocation")]
    footprint: Footprint,
    #[cfg(feature = "allocation")]
    retention: Retention,
}

#[cfg(feature = "allocation")]
#[derive(Serialize)]
struct Retention {
    engine: i128,
    report: i128,
    encoded: i128,
}

#[cfg(feature = "allocation")]
#[derive(Serialize)]
pub struct Footprint {
    allocated: usize,
    released: usize,
    retained: i128,
    peak: i128,
}

pub fn measure<Value: Engine>(
    initialize: impl FnOnce() -> Value,
    budget: usize,
) -> Result<Record, Failure> {
    let (mut engine, initialization) = meter::measure(initialize);
    let ((), execution) = meter::measure(|| {
        engine.execute(budget, crate::limit::LARGE);
    });
    engine.finish()?;
    let (report, reporting) = meter::measure(|| black_box(engine.report()));
    let mut observation = Value::observe(&report);
    let (encoded, serialization) = meter::measure(|| serde_json::to_vec(&report));
    let encoded = encoded.map_err(Failure::Encoding)?;
    observation.byte = encoded.len();
    let mut fingerprint = Fingerprint::default();
    fingerprint.update(&encoded);
    observation.fingerprint = fingerprint.value();
    #[cfg(not(feature = "allocation"))]
    let ((), release) = meter::measure(|| drop((engine, report, encoded)));
    #[cfg(feature = "allocation")]
    let (retention, release) = meter::measure(|| {
        let retained = || crate::allocation::retained() as i128;
        let before = retained();
        drop(engine);
        let engine = retained();
        drop(report);
        let report = retained();
        drop(encoded);
        let encoded = retained();
        Retention {
            engine: before - engine,
            report: engine - report,
            encoded: report - encoded,
        }
    });
    #[cfg(feature = "allocation")]
    let footprint = Footprint::new([
        &initialization,
        &execution,
        &reporting,
        &serialization,
        &release,
    ]);
    let duration = initialization.duration
        + execution.duration
        + reporting.duration
        + serialization.duration
        + release.duration;
    Ok(Record {
        initialization,
        execution,
        reporting,
        serialization,
        release,
        duration,
        observation,
        #[cfg(feature = "allocation")]
        footprint,
        #[cfg(feature = "allocation")]
        retention,
    })
}

#[cfg(feature = "allocation")]
impl Footprint {
    pub fn new<'value>(measurement: impl IntoIterator<Item = &'value Measurement>) -> Self {
        let mut footprint = Self {
            allocated: 0,
            released: 0,
            retained: 0,
            peak: 0,
        };
        for measurement in measurement {
            let allocation = &measurement.allocation;
            footprint.allocated += allocation.allocated;
            footprint.released += allocation.released;
            footprint.peak = footprint
                .peak
                .max(footprint.retained + allocation.peak as i128);
            footprint.retained += allocation.retained;
        }
        footprint
    }
}
