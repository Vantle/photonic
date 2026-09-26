#![deny(unsafe_code)]

#[cfg(target_os = "macos")]
mod adam;
#[cfg(target_os = "macos")]
mod batch;
#[cfg(target_os = "macos")]
mod delta;
#[cfg(target_os = "macos")]
pub mod engine;
#[cfg(not(target_os = "macos"))]
#[path = "absent.rs"]
pub mod engine;
pub mod failure;
#[cfg(target_os = "macos")]
mod occurrence;
#[cfg(target_os = "macos")]
mod operand;
#[cfg(target_os = "macos")]
mod pipeline;
#[cfg(target_os = "macos")]
mod plain;
#[cfg(target_os = "macos")]
mod runtime;
#[cfg(target_os = "macos")]
mod trace;

#[cfg(test)]
#[path = "test/suite.rs"]
mod test;
