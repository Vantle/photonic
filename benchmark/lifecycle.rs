use crate::meter::{self, Measurement};
use photonic::runtime::Limit;
use serde::Serialize;
use std::hash::{Hash, Hasher};
use std::hint::black_box;

pub trait Engine {
    type Report: Serialize;

    fn execute(&mut self, budget: usize, limit: Limit);
    fn report(&self) -> Self::Report;
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

    fn report(&self) -> Self::Report {
        self.report()
    }

    fn observe(report: &Self::Report) -> Observation {
        assert_eq!(report.outcome, photonic::prism::Outcome::Reached);
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
        self.run(budget, Some(limit));
    }

    fn report(&self) -> Self::Report {
        self.snapshot()
    }

    fn observe(report: &Self::Report) -> Observation {
        assert!(report.closed);
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
}

#[cfg(feature = "allocation")]
#[derive(Serialize)]
struct Footprint {
    allocated: usize,
    released: usize,
    retained: i128,
    peak: i128,
}

pub fn measure<Value: Engine>(initialize: impl FnOnce() -> Value, budget: usize) -> Record {
    let (mut engine, initialization) = meter::measure(initialize);
    let ((), execution) = meter::measure(|| {
        engine.execute(
            budget,
            Limit {
                state: 262_144,
                record: 100_000_000,
                world: 1024,
                cell: 16_384,
                frame: 2048,
            },
        );
    });
    let (report, reporting) = meter::measure(|| black_box(engine.report()));
    let mut observation = Value::observe(&report);
    let (encoded, serialization) = meter::measure(|| serde_json::to_vec(&report).unwrap());
    observation.byte = encoded.len();
    let mut fingerprint = std::collections::hash_map::DefaultHasher::new();
    encoded.hash(&mut fingerprint);
    observation.fingerprint = fingerprint.finish();
    let ((), release) = meter::measure(|| drop((engine, report, encoded)));
    #[cfg(feature = "allocation")]
    let footprint = {
        let mut footprint = Footprint {
            allocated: 0,
            released: 0,
            retained: 0,
            peak: 0,
        };
        for measurement in [
            &initialization,
            &execution,
            &reporting,
            &serialization,
            &release,
        ] {
            let allocation = &measurement.allocation;
            footprint.allocated += allocation.allocated;
            footprint.released += allocation.released;
            footprint.peak = footprint
                .peak
                .max(footprint.retained + allocation.peak as i128);
            footprint.retained += allocation.retained;
        }
        footprint
    };
    let duration = initialization.duration
        + execution.duration
        + reporting.duration
        + serialization.duration
        + release.duration;
    Record {
        initialization,
        execution,
        reporting,
        serialization,
        release,
        duration,
        observation,
        #[cfg(feature = "allocation")]
        footprint,
    }
}
