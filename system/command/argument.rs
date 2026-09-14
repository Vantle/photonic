use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(version, about = "Photonic language tools")]
pub struct Argument {
    #[command(subcommand)]
    pub operation: Operation,
}

#[derive(Subcommand)]
pub enum Operation {
    #[command(about = "Parse source structure and print its syntax tree")]
    Parse { path: PathBuf },
    #[command(about = "Execute a Photonic program with bounded graph exploration")]
    Run {
        path: PathBuf,
        #[command(flatten)]
        execution: Execution,
    },
    #[command(about = "Check exact configuration reachability with Obsidian")]
    Obsidian {
        path: PathBuf,
        #[arg(
            long,
            help = "Target configuration file, without additional declarations"
        )]
        target: PathBuf,
        #[arg(
            long = "path",
            help = "Follow one direct execution path; failure remains unknown"
        )]
        walk: bool,
        #[command(flatten)]
        execution: Execution,
    },
}

#[derive(Args)]
pub struct Execution {
    #[arg(
        long,
        help = "Load a declaration-only Photonic library; repeat for each source file"
    )]
    pub library: Vec<PathBuf>,
    #[arg(long = "steps", default_value_t = 12_000)]
    pub step: usize,
    #[arg(long = "workers", default_value_t = 1)]
    pub worker: usize,
    #[arg(long = "records", default_value_t = 1_000_000)]
    pub record: usize,
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
        help = "Input format; .json selects JSON, .particle and .wave use identical Photonic syntax"
    )]
    pub format: Option<Format>,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum Format {
    Photonic,
    Json,
}
