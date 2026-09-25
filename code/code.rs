#![forbid(unsafe_code)]

pub mod analogy;
pub mod atom;
pub mod canonical;
pub mod configuration;
pub mod distance;
pub mod forest;
pub mod hashing;
pub mod observation;
pub mod output;
pub mod particle;
pub mod program;
pub mod rule;
pub mod tree;
pub mod value;

#[cfg(test)]
#[path = "test/suite.rs"]
mod test;
