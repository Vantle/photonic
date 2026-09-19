#![forbid(unsafe_code)]

pub use frontend::{failure, lowering, parser, source, syntax};

mod accumulator;
mod activation;
mod agenda;
mod application;
mod assignment;
mod basis;
mod canonical;
mod catalog;
pub mod executor;
mod fingerprint;
pub mod flow;
mod gate;
mod graph;
mod hashing;
mod incidence;
mod index;
mod layout;
mod link;
mod ordering;
mod particle;
mod partition;
pub mod path;
mod plan;
mod prefix;
pub mod prism;
mod program;
mod propagation;
mod query;
mod reduction;
mod refinement;
mod relation;
pub mod runtime;
mod search;
mod selection;
mod slot;
pub mod snapshot;
mod state;
mod structure;
pub mod support;
mod symmetry;
mod term;
mod work;

#[cfg(test)]
#[path = "test/suite.rs"]
mod test;

#[cfg(feature = "measurement")]
pub mod measurement;
