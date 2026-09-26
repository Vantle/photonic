#![forbid(unsafe_code)]

mod coherence;
pub mod event;
pub mod exploration;
pub mod flat;
pub mod limit;
pub mod schedule;
pub mod state;
pub mod walk;

#[cfg(test)]
#[path = "test/suite.rs"]
mod test;
