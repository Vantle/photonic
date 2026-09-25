#![forbid(unsafe_code)]

pub mod comparison;
mod graph;
pub mod group;
mod partition;
pub mod rename;
pub mod search;
pub mod structure;
mod twin;

#[cfg(test)]
#[path = "test/suite.rs"]
mod test;
