use std::time::{Duration, Instant};

// Repeating a run for a tenth of a second first settles the allocator and the caches, so the
// measured samples do not include the first touches of memory.
pub fn warm<Failure>(mut run: impl FnMut() -> Result<(), Failure>) -> Result<(), Failure> {
    let start = Instant::now();
    loop {
        run()?;
        if start.elapsed() >= Duration::from_millis(100) {
            return Ok(());
        }
    }
}
