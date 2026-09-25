#![forbid(unsafe_code)]

pub mod archive;
pub mod bar;
pub mod corpus;
pub mod curriculum;
pub mod demonstration;
pub mod edit;
pub mod encoding;
pub mod export;
pub mod guide;
pub mod home;
pub mod import;
pub mod judge;
pub mod objective;
pub mod play;
pub mod pool;
pub mod problem;
pub mod renewal;
pub mod replay;
pub mod search;
pub mod server;
pub mod session;
pub mod solution;
pub mod synthetic;
pub mod task;
pub mod train;

#[cfg(test)]
#[path = "test/suite.rs"]
mod test;
