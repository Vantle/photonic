#![forbid(unsafe_code)]

pub mod atom;
pub mod canonical;
pub mod configuration;
pub mod forest;
pub mod observation;
pub mod output;
pub mod particle;
pub mod program;
pub mod rule;
pub mod scope;
pub mod value;

#[cfg(test)]
#[path = "test/suite.rs"]
mod test;
