#![forbid(unsafe_code)]

pub use frontend::{failure, lowering, parser, source, syntax};

mod accumulator;
mod agenda;
mod application;
mod arena;
mod assignment;
mod basis;
mod canonical;
mod catalog;
mod change;
mod dispatch;
pub mod executor;
mod fingerprint;
pub mod flow;
mod gate;
mod graph;
mod hashing;
mod incidence;
mod index;
mod joining;
mod layout;
mod link;
mod membership;
mod ordering;
mod particle;
mod partition;
pub mod path;
mod plan;
mod position;
mod prefix;
pub mod prism;
mod program;
mod propagation;
mod reachability;
mod reader;
mod recipe;
mod reduction;
mod refinement;
mod relation;
mod replay;
mod rewrite;
pub mod runtime;
mod search;
mod selection;
mod sequence;
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
