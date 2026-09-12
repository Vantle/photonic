use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

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
    #[command(about = "Execute a Molten program with bounded graph exploration")]
    Run {
        path: PathBuf,
        #[command(flatten)]
        execution: Execution,
    },
}

#[derive(Args)]
pub struct Execution {
    #[arg(long = "steps", default_value_t = 12_000)]
    pub step: usize,
    #[arg(long = "states", default_value_t = 80)]
    pub state: usize,
    #[arg(long = "cells", default_value_t = 12)]
    pub cell: usize,
    #[arg(long = "frames", default_value_t = 10)]
    pub frame: usize,
    #[arg(long = "coherences", default_value_t = 4)]
    pub coherence: usize,
    #[arg(long, help = "Print the complete execution report as JSON")]
    pub json: bool,
    #[arg(
        long,
        value_enum,
        help = "Input format; inferred from the .json extension otherwise"
    )]
    pub format: Option<Format>,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum Format {
    Molten,
    Json,
}
