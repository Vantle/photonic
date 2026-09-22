use crate::lifecycle::Engine;
use crate::meter::{self, Measurement};
use clap::ValueEnum;
use serde::Serialize;
use std::hash::Hasher;
use std::io::Write;

#[derive(Clone, Copy, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Owned,
    View,
}

struct Output {
    buffer: Option<Vec<u8>>,
    length: usize,
    fingerprint: std::collections::hash_map::DefaultHasher,
}

impl Write for Output {
    fn write(&mut self, value: &[u8]) -> std::io::Result<usize> {
        if let Some(buffer) = &mut self.buffer {
            buffer.extend_from_slice(value);
        }
        self.length += value.len();
        self.fingerprint.write(value);
        Ok(value.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[derive(Serialize)]
pub struct Record {
    mode: Mode,
    writer: bool,
    initialization: Measurement,
    execution: Measurement,
    export: Measurement,
    release: Measurement,
    duration: f64,
    byte: usize,
    fingerprint: u64,
    #[cfg(feature = "measurement")]
    phase: Vec<photonic::measurement::profile::Measurement>,
    #[cfg(feature = "allocation")]
    footprint: crate::lifecycle::Footprint,
}

pub fn measure<Value: Engine>(
    initialize: impl FnOnce() -> Value,
    budget: usize,
    mode: Mode,
    writer: bool,
) -> Record {
    let (mut engine, initialization) = meter::measure(initialize);
    let ((), execution) = meter::measure(|| engine.execute(budget, crate::lifecycle::limit()));
    #[cfg(feature = "measurement")]
    photonic::measurement::profile::take();
    let (output, export) = meter::measure(|| {
        let mut output = Output {
            buffer: (!writer).then(Vec::new),
            length: 0,
            fingerprint: Default::default(),
        };
        match mode {
            Mode::Owned => serde_json::to_writer(&mut output, &engine.report()).unwrap(),
            Mode::View => serde_json::to_writer(&mut output, &engine.view()).unwrap(),
        }
        output
    });
    #[cfg(feature = "measurement")]
    let phase = photonic::measurement::profile::take();
    let byte = output.length;
    let fingerprint = output.fingerprint.finish();
    let ((), release) = meter::measure(|| drop((engine, output)));
    let duration =
        initialization.duration + execution.duration + export.duration + release.duration;
    Record {
        mode,
        writer,
        #[cfg(feature = "allocation")]
        footprint: crate::lifecycle::Footprint::new([
            &initialization,
            &execution,
            &export,
            &release,
        ]),
        initialization,
        execution,
        export,
        release,
        duration,
        byte,
        fingerprint,
        #[cfg(feature = "measurement")]
        phase,
    }
}
