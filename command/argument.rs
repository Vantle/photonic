use clap::{Args, Parser, Subcommand};
use std::num::NonZeroUsize;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about = "Photonic language tools")]
pub struct Argument {
    #[command(subcommand)]
    pub operation: Operation,
}

#[derive(Subcommand)]
pub enum Operation {
    #[command(about = "Lower a program, with its libraries, into a program value as JSON")]
    Lower(Lower),
    #[command(about = "Explore every future with the runtime and list its configurations")]
    Run(Run),
    #[command(
        about = "Check whether an exact target configuration is reachable, as photonic_test does"
    )]
    Prism(Prism),
    #[command(
        about = "Check a program: diagnostics, then claims answered holds, fails or unknown; exits 1 unless all hold"
    )]
    Check(Check),
    #[command(
        about = "Explore every future and summarize configurations, end configurations and rules"
    )]
    Explore(Explore),
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
    #[command(
        about = "Compare two programs by the configurations and events they reach; exits 1 unless both close and agree"
    )]
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

#[derive(Args, Default)]
pub struct Source {
    #[arg(long, help = "Load a declaration-only library file; repeat for each")]
    pub library: Vec<PathBuf>,
    #[arg(long, help = "Photonic source added after the files")]
    pub source: Option<String>,
}

#[derive(Args)]
pub struct Budget {
    #[arg(long, default_value_t = spectrum::budget::Budget::default().work, help = "Work steps before the search stops")]
    pub work: usize,
    #[arg(long, default_value_t = spectrum::budget::Budget::default().configuration, help = "Configurations kept")]
    pub configuration: usize,
    #[arg(long, default_value_t = spectrum::budget::Budget::default().coherence, help = "Coherences in one configuration")]
    pub coherence: usize,
    #[arg(long, default_value_t = spectrum::budget::Budget::default().occurrence, help = "Occurrences in one configuration")]
    pub occurrence: usize,
    #[arg(long, default_value_t = spectrum::budget::Budget::default().scope, help = "Scopes in one configuration")]
    pub scope: usize,
    #[arg(long, default_value_t = spectrum::budget::Budget::default().record, help = "Records the engine retains")]
    pub record: usize,
}

impl From<&Budget> for spectrum::budget::Budget {
    fn from(flag: &Budget) -> Self {
        Self {
            work: flag.work,
            configuration: flag.configuration,
            coherence: flag.coherence,
            occurrence: flag.occurrence,
            scope: flag.scope,
            record: flag.record,
        }
    }
}

#[derive(Args)]
pub struct Print {
    #[arg(long, help = "Print the answer as a JSON envelope")]
    pub json: bool,
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
pub struct Lower {
    #[arg(
        required = true,
        help = "Program files: .wave or .particle source, or .json programs assembled by Bazel"
    )]
    pub file: Vec<PathBuf>,
    #[command(flatten)]
    pub source: Source,
}

#[derive(Args)]
pub struct Run {
    #[arg(
        required = true,
        help = "Program files: .wave or .particle source, or .json programs assembled by Bazel"
    )]
    pub file: Vec<PathBuf>,
    #[command(flatten)]
    pub source: Source,
    #[command(flatten)]
    pub budget: Budget,
    #[arg(long, default_value_t = NonZeroUsize::MIN, help = "Threads that explore in parallel")]
    pub worker: NonZeroUsize,
    #[arg(long, help = "Print the complete execution report as JSON")]
    pub json: bool,
    #[arg(long, requires = "json", help = "Serialize JSON without indentation")]
    pub compact: bool,
}

#[derive(Args)]
pub struct Prism {
    #[command(flatten)]
    pub run: Run,
    #[arg(long, help = "A file holding the complete target configuration")]
    pub target: PathBuf,
    #[arg(
        long,
        help = "Follow one direct execution path; failure to reach the target stays unknown"
    )]
    pub path: bool,
}

#[derive(Args)]
pub struct Check {
    #[arg(help = "Program files: .wave or .particle source, or .json programs assembled by Bazel")]
    pub file: Vec<PathBuf>,
    #[command(flatten)]
    pub source: Source,
    #[command(flatten)]
    pub budget: Budget,
    #[arg(
        long,
        help = "Follow one direct path in source order instead of exploring every future"
    )]
    pub path: bool,
    #[arg(
        long,
        help = "In path mode, the complete configuration the path stops at"
    )]
    pub goal: Option<String>,
    #[command(flatten)]
    pub claim: Claim,
    #[command(flatten)]
    pub print: Print,
}

#[derive(Args)]
pub struct Explore {
    #[arg(help = "Program files: .wave or .particle source, or .json programs assembled by Bazel")]
    pub file: Vec<PathBuf>,
    #[command(flatten)]
    pub source: Source,
    #[command(flatten)]
    pub budget: Budget,
    #[arg(
        long,
        help = "Follow one direct path in source order instead of exploring every future"
    )]
    pub path: bool,
    #[arg(
        long,
        help = "In path mode, the complete configuration the path stops at"
    )]
    pub goal: Option<String>,
    #[arg(long, help = "With the goal, it also lists every loaded root rule")]
    pub preserve: bool,
    #[arg(long, default_value_t = spectrum::explore::LIMIT, help = "End configurations listed")]
    pub limit: usize,
    #[command(flatten)]
    pub print: Print,
}

#[derive(Args)]
pub struct Select {
    #[arg(help = "Program files")]
    pub file: Vec<PathBuf>,
    #[arg(long, help = "A Photonic pattern: B, B.X, B, C, ([A] B) or [B, C] D")]
    pub pattern: String,
    #[arg(long, default_value_t = spectrum::select::LIMIT, help = "Matches listed")]
    pub limit: usize,
    #[arg(long, default_value_t = 0, help = "Matches skipped")]
    pub offset: usize,
    #[command(flatten)]
    pub source: Source,
    #[command(flatten)]
    pub budget: Budget,
    #[arg(
        long,
        help = "Follow one direct path in source order instead of exploring every future"
    )]
    pub path: bool,
    #[command(flatten)]
    pub print: Print,
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
    #[arg(
        long,
        help = "Follow one direct path in source order instead of exploring every future"
    )]
    pub path: bool,
    #[command(flatten)]
    pub print: Print,
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
    #[arg(long, default_value_t = spectrum::miss::LIMIT, help = "Configurations listed")]
    pub limit: usize,
    #[command(flatten)]
    pub source: Source,
    #[command(flatten)]
    pub budget: Budget,
    #[arg(
        long,
        help = "Follow one direct path in source order instead of exploring every future"
    )]
    pub path: bool,
    #[command(flatten)]
    pub print: Print,
}

#[derive(Args)]
pub struct Compare {
    #[arg(help = "The program before")]
    pub left: PathBuf,
    #[arg(help = "The program after")]
    pub right: PathBuf,
    #[arg(long, default_value_t = spectrum::compare::LIMIT, help = "Differences listed on each side")]
    pub limit: usize,
    #[command(flatten)]
    pub source: Source,
    #[command(flatten)]
    pub budget: Budget,
    #[arg(
        long,
        help = "Follow one direct path in source order instead of exploring every future"
    )]
    pub path: bool,
    #[command(flatten)]
    pub claim: Claim,
    #[command(flatten)]
    pub print: Print,
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
        default_value_t = spectrum::shape::NODE,
        help = "Search tree nodes the symmetry engine may visit"
    )]
    pub node: usize,
    #[command(flatten)]
    pub source: Source,
    #[command(flatten)]
    pub print: Print,
}
