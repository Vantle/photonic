#![deny(unsafe_code)]

#[cfg(target_os = "macos")]
mod arena;
#[cfg(target_os = "macos")]
pub mod engine;
#[cfg(not(target_os = "macos"))]
#[path = "absent.rs"]
pub mod engine;
pub mod failure;
#[cfg(target_os = "macos")]
mod grow;
#[cfg(target_os = "macos")]
mod hash;
#[cfg(target_os = "macos")]
mod pass;
#[cfg(target_os = "macos")]
mod scan;
#[cfg(target_os = "macos")]
mod setting;
pub mod shape;
#[cfg(target_os = "macos")]
mod store;
#[cfg(target_os = "macos")]
mod table;
#[cfg(target_os = "macos")]
mod upload;
#[cfg(target_os = "macos")]
mod window;
#[cfg(target_os = "macos")]
mod work;

#[cfg(test)]
#[path = "test/suite.rs"]
mod test;
