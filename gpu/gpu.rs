#![deny(unsafe_code)]

mod adam;
mod batch;
mod delta;
pub mod engine;
pub mod failure;
mod occurrence;
pub mod operand;
mod pipeline;
pub mod plain;
mod seal;
mod trace;

#[cfg(target_os = "macos")]
pub mod runtime;

#[cfg(not(target_os = "macos"))]
#[path = "absent.rs"]
pub mod runtime;

#[cfg(test)]
#[path = "test/suite.rs"]
mod test;
