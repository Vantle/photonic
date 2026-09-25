#![forbid(unsafe_code)]

pub mod analysis;
pub mod comparison;
mod graph;
pub mod group;
mod measure;
mod partition;
pub mod pattern;
mod rename;
pub mod search;
pub mod statement;
pub mod structure;
mod twin;

#[cfg(test)]
#[path = "test/suite.rs"]
mod test;
