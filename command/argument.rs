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
    #[command(
        about = "Check a program: diagnostics, then claims answered holds, fails or unknown; exits 1 unless all hold"
    )]
    Check(Question),
    #[command(
        about = "Explore every future and summarize configurations, end configurations, rules and claims"
    )]
    Explore(Question),
    #[command(about = "Find the configurations or events that match a Photonic pattern")]
    Select(Select),
    #[command(
        about = "Describe a rule, configuration, coherence, occurrence, frame or event by its handle"
    )]
    Inspect(Pointer),
    #[command(about = "Explain why a configuration, event or occurrence is here")]
    Cause(Pointer),
    #[command(
        about = "Explain why not: the configurations nearest a target, or why a rule does not fire"
    )]
    Miss(Miss),
    #[command(about = "List the events that can happen at a configuration")]
    Step(Pointer),
    #[command(about = "Compare two programs by the configurations and events they reach")]
    Compare(Compare),
    #[command(
        about = "Describe a program up to the names of its atoms, or group programs by shape"
    )]
    Shape(Shape),
    #[command(
        about = "Serve Spectrum to agents over the Model Context Protocol on standard input and output"
    )]
    Mcp,
}

#[derive(Args)]
pub struct Source {
    #[arg(long, help = "Load a declaration-only library file; repeat for each")]
    pub library: Vec<PathBuf>,
    #[arg(long, help = "Photonic source added after the files")]
    pub source: Option<String>,
    #[arg(long, help = "Print the answer as a JSON envelope")]
    pub json: bool,
}

#[derive(Args)]
pub struct Budget {
    #[arg(long, help = "Work steps before the search stops")]
    pub work: Option<usize>,
    #[arg(long, help = "Configurations kept")]
    pub configuration: Option<usize>,
    #[arg(long, help = "Coherences in one configuration")]
    pub coherence: Option<usize>,
    #[arg(long, help = "Occurrences in one configuration")]
    pub occurrence: Option<usize>,
    #[arg(long, help = "Scopes in one configuration")]
    pub scope: Option<usize>,
    #[arg(long, help = "Records the engine retains")]
    pub record: Option<usize>,
    #[arg(
        long,
        help = "Follow one direct path in source order instead of exploring every future"
    )]
    pub path: bool,
}

#[derive(Args)]
pub struct Claim {
    #[arg(long, help = "Claim that some configuration matches this pattern")]
    pub reach: Vec<String>,
    #[arg(long, help = "Claim that no configuration matches this pattern")]
    pub avoid: Vec<String>,
    #[arg(long, help = "Claim that every configuration matches this pattern")]
    pub always: Vec<String>,
    #[arg(
        long,
        help = "Claim that every run reaches a configuration matching this pattern"
    )]
    pub inevitable: Vec<String>,
    #[arg(long, help = "Claim that every end configuration matches this pattern")]
    pub outcome: Vec<String>,
    #[arg(
        long,
        help = "Read reach and avoid patterns as complete configurations, as Prism does"
    )]
    pub exact: bool,
    #[arg(
        long,
        help = "Exact targets and the goal also list every loaded root rule"
    )]
    pub preserve: bool,
}

#[derive(Args)]
pub struct Question {
    #[arg(help = "Program files: .wave or .particle source, or .json programs assembled by Bazel")]
    pub file: Vec<PathBuf>,
    #[command(flatten)]
    pub source: Source,
    #[command(flatten)]
    pub budget: Budget,
    #[command(flatten)]
    pub claim: Claim,
    #[arg(
        long,
        help = "In path mode, the complete configuration the path stops at"
    )]
    pub goal: Option<String>,
}

#[derive(Args)]
pub struct Select {
    #[arg(help = "Program files")]
    pub file: Vec<PathBuf>,
    #[arg(long, help = "A Photonic pattern: B, B.X, B, C, ([A] B) or [B, C] D")]
    pub pattern: String,
    #[arg(long, default_value_t = 20, help = "Matches listed")]
    pub limit: usize,
    #[arg(long, default_value_t = 0, help = "Matches skipped")]
    pub offset: usize,
    #[command(flatten)]
    pub source: Source,
    #[command(flatten)]
    pub budget: Budget,
}

#[derive(Args)]
pub struct Pointer {
    #[arg(
        required = true,
        help = "Program files, then a handle such as r2, s11, e12, s11.c0, s11.o1 or s10.f1"
    )]
    pub argument: Vec<String>,
    #[command(flatten)]
    pub source: Source,
    #[command(flatten)]
    pub budget: Budget,
}

#[derive(Args)]
pub struct Miss {
    #[arg(
        required = true,
        help = "Program files, then optionally a rule handle such as r3"
    )]
    pub argument: Vec<String>,
    #[arg(
        long,
        help = "A pattern, or with --exact a complete configuration, to reach"
    )]
    pub target: Option<String>,
    #[arg(
        long,
        help = "Compare whole configurations as Prism does: coherences, live rules and open scopes"
    )]
    pub exact: bool,
    #[arg(
        long,
        help = "With --exact, the target also lists every loaded root rule"
    )]
    pub preserve: bool,
    #[arg(long, default_value_t = 3, help = "Configurations listed")]
    pub limit: usize,
    #[command(flatten)]
    pub source: Source,
    #[command(flatten)]
    pub budget: Budget,
}

#[derive(Args)]
pub struct Compare {
    #[arg(help = "The program before")]
    pub left: PathBuf,
    #[arg(help = "The program after")]
    pub right: PathBuf,
    #[arg(long, default_value_t = 12, help = "Differences listed on each side")]
    pub limit: usize,
    #[command(flatten)]
    pub source: Source,
    #[command(flatten)]
    pub budget: Budget,
    #[command(flatten)]
    pub claim: Claim,
}

#[derive(Args)]
pub struct Shape {
    #[arg(
        required = true,
        help = "One program to describe, or several to group by shape"
    )]
    pub file: Vec<PathBuf>,
    #[arg(
        long,
        help = "A target configuration every renaming must also preserve"
    )]
    pub target: Option<String>,
    #[arg(
        long,
        help = "Keep this atom's name in every renaming; repeat for each atom"
    )]
    pub fix: Vec<String>,
    #[arg(
        long,
        default_value_t = 1_000_000,
        help = "Search tree nodes the symmetry engine may visit"
    )]
    pub node: usize,
    #[command(flatten)]
    pub source: Source,
}

#[derive(Args)]
pub struct Execution {
    #[arg(
        long,
        help = "Load a declaration-only Photonic library; repeat for each source file"
    )]
    pub library: Vec<PathBuf>,
    #[arg(
        long,
        default_value_t = 12_000,
        help = "Work steps before the search stops"
    )]
    pub work: usize,
    #[arg(long, default_value_t = Limit::default().state, help = "Configurations kept")]
    pub configuration: usize,
    #[arg(long, default_value_t = Limit::default().world, help = "Coherences in one configuration")]
    pub coherence: usize,
    #[arg(long, default_value_t = Limit::default().cell, help = "Occurrences in one configuration")]
    pub occurrence: usize,
    #[arg(long, default_value_t = Limit::default().frame, help = "Scopes in one configuration")]
    pub scope: usize,
    #[arg(long, default_value_t = Limit::default().record, help = "Records the engine retains")]
    pub record: usize,
    #[arg(long, default_value_t = 1, help = "Threads that explore in parallel")]
    pub worker: usize,
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
