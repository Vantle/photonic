#![forbid(unsafe_code)]

mod analogy;
pub mod archive;
pub mod attempt;
mod bar;
mod corpus;
pub mod curriculum;
mod demonstration;
mod distance;
pub mod edit;
pub mod encoding;
pub mod export;
pub mod guide;
pub mod home;
pub mod import;
mod judge;
pub mod objective;
pub mod play;
pub mod pool;
pub mod problem;
mod renewal;
mod replay;
pub mod search;
mod server;
pub mod session;
pub mod solution;
mod synthetic;
pub mod task;
pub mod train;
mod tree;

#[cfg(test)]
#[path = "test/suite.rs"]
mod test;
