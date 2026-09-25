#![forbid(unsafe_code)]

pub mod analysis;
pub mod comparison;
mod forest;
mod graph;
pub mod group;
mod partition;
pub mod pattern;
pub mod rename;
pub mod search;
pub mod statement;
pub mod structure;
mod twin;

#[cfg(test)]
#[path = "test/suite.rs"]
mod test;
