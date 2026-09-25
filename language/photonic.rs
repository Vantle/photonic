#![forbid(unsafe_code)]

pub use frontend::{lowering, parser, source, syntax, text};

mod accumulator;
mod affected;
mod agenda;
mod application;
mod arena;
mod assignment;
mod basis;
mod budget;
mod candidate;
mod canonical;
mod catalog;
mod change;
mod consumption;
mod delta;
mod dispatch;
mod evaluation;
pub mod execution;
pub mod executor;
mod factor;
mod fingerprint;
mod flow;
mod gate;
mod graph;
mod hashing;
mod incidence;
mod index;
mod joining;
mod layout;
mod lexical;
mod link;
mod location;
mod mask;
mod membership;
mod ordering;
mod particle;
mod partition;
pub mod path;
mod pattern;
pub mod place;
mod plan;
mod population;
mod position;
mod prefix;
mod preparation;
pub mod prism;
#[cfg(feature = "measurement")]
pub mod profile;
#[cfg(not(feature = "measurement"))]
mod profile;
mod program;
mod proof;
mod propagation;
mod reachability;
mod reader;
mod reduction;
mod refinement;
mod relation;
mod render;
mod replay;
mod revision;
pub mod runtime;
mod search;
mod selection;
mod sequence;
mod slot;
pub mod snapshot;
mod state;
pub mod status;
mod structure;
mod support;
mod symmetry;
mod term;
mod work;

#[cfg(test)]
#[path = "test/suite.rs"]
mod test;

#[cfg(feature = "measurement")]
pub mod measurement;

#[cfg(test)]
#[path = "test/report.rs"]
mod report;
