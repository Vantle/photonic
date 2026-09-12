use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about = "Molten language tools")]
pub struct Argument {
    #[command(subcommand)]
    pub operation: Operation,
}

#[derive(Subcommand)]
pub enum Operation {
    #[command(about = "Parse source structure and print its syntax tree")]
    Parse { path: PathBuf },
}
