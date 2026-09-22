use serde::Serialize;
use std::time::Instant;

#[derive(Serialize)]
pub struct Measurement {
    pub duration: f64,
    #[cfg(feature = "allocation")]
    pub allocation: crate::allocation::Measurement,
}

pub fn measure<Value>(run: impl FnOnce() -> Value) -> (Value, Measurement) {
    let start = Instant::now();
    #[cfg(feature = "allocation")]
    let (value, allocation) = crate::allocation::measure(run);
    #[cfg(not(feature = "allocation"))]
    let value = run();
    (
        value,
        Measurement {
            duration: start.elapsed().as_secs_f64(),
            #[cfg(feature = "allocation")]
            allocation,
        },
    )
}
