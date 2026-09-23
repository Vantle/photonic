use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use photonic::runtime::Limit;

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
    #[command(about = "Lower Photonic source into a program value as JSON")]
    Lower {
        path: PathBuf,
        #[arg(
            long,
            help = "Append the root rules from this program; repeat for each context source"
        )]
        context: Vec<PathBuf>,
    },
    #[command(about = "Execute a Photonic program with bounded graph exploration")]
    Run {
        path: PathBuf,
        #[command(flatten)]
        execution: Execution,
    },
    #[command(about = "Check exact configuration reachability with Prism")]
    Prism {
        path: PathBuf,
        #[arg(long, help = "Complete target state, including live rule occurrences")]
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
    #[arg(long = "records", default_value_t = Limit::default().record)]
    pub record: usize,
    #[arg(long = "states", default_value_t = Limit::default().state)]
    pub state: usize,
    #[arg(long = "cells", default_value_t = Limit::default().cell)]
    pub cell: usize,
    #[arg(long = "frames", default_value_t = Limit::default().frame)]
    pub frame: usize,
    #[arg(long = "coherences", default_value_t = Limit::default().world)]
    pub coherence: usize,
    #[arg(long, help = "Print the complete execution report as JSON")]
    pub json: bool,
    #[arg(long, requires = "json", help = "Serialize JSON without indentation")]
    pub compact: bool,
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
