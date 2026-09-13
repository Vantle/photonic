#![forbid(unsafe_code)]

mod agenda;
mod expansion;
mod symmetry;
mod work;

mod canonical;
pub mod executor;
pub mod failure;
pub mod flow;
pub mod lowering;
mod matching;
pub mod obsidian;
mod ordering;
pub mod parser;
pub mod particle;
pub mod path;
mod program;
mod refinement;
pub mod rule;
pub mod runtime;
mod search;
pub mod snapshot;
pub mod source;
mod state;
pub mod support;
pub mod syntax;

#[cfg(test)]
#[path = "test/suite.rs"]
mod test;

#[cfg(feature = "measurement")]
pub mod measurement;
