#![deny(unsafe_code)]

#[cfg(target_os = "macos")]
pub mod device;
pub mod failure;
#[cfg(target_os = "macos")]
pub mod operand;
pub mod plain;

#[cfg(test)]
#[path = "test/suite.rs"]
mod test;
