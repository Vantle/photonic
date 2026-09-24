#![forbid(unsafe_code)]

pub mod emit;
pub mod execution;
pub mod failure;
pub mod lift;
pub mod text;
pub mod vocabulary;

#[cfg(test)]
#[path = "test/suite.rs"]
mod test;
