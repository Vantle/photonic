use crate::engine::{Engine, Failure};
use crate::fingerprint::Fingerprint;
use crate::meter::{self, Measurement};
use clap::ValueEnum;
use serde::Serialize;
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
    fingerprint: Fingerprint,
}

impl Write for Output {
    fn write(&mut self, value: &[u8]) -> std::io::Result<usize> {
        if let Some(buffer) = &mut self.buffer {
            buffer.extend_from_slice(value);
        }
        self.length += value.len();
        self.fingerprint.update(value);
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
    #[cfg(feature = "allocation")]
    footprint: crate::engine::Footprint,
}

pub fn measure<Value: Engine>(
    initialize: impl FnOnce() -> Value,
    budget: usize,
    mode: Mode,
    writer: bool,
) -> Result<Record, Failure> {
    let (mut engine, initialization) = meter::measure(initialize);
    let ((), execution) = meter::measure(|| engine.execute(budget, crate::limit::LARGE));
    engine.finish()?;
    let (output, export) = meter::measure(|| {
        let mut output = Output {
            buffer: (!writer).then(Vec::new),
            length: 0,
            fingerprint: Fingerprint::default(),
        };
        let written = match mode {
            Mode::Owned => serde_json::to_writer(&mut output, &engine.report()),
            Mode::View => serde_json::to_writer(&mut output, &engine.stream()),
        };
        written.map(|()| output)
    });
    let output = output.map_err(Failure::Encoding)?;
    let byte = output.length;
    let fingerprint = output.fingerprint.value();
    let ((), release) = meter::measure(|| drop((engine, output)));
    let duration =
        initialization.duration + execution.duration + export.duration + release.duration;
    Ok(Record {
        mode,
        writer,
        #[cfg(feature = "allocation")]
        footprint: crate::engine::Footprint::new([&initialization, &execution, &export, &release]),
        initialization,
        execution,
        export,
        release,
        duration,
        byte,
        fingerprint,
    })
}
