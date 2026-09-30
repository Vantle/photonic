use clap::{
    Arg, ArgAction, ArgGroup, ArgMatches, Args, Command, FromArgMatches, Parser, Subcommand,
};
use spectrum::claim::Kind;
use spectrum::failure::{Code, Failure};
use spectrum::recording::Engine;
use spectrum::subject::Subject;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};

const FILE: &str = "Program files: .wave or .particle source, or .json programs assembled by Bazel";

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
    #[command(
        about = "List every configuration a program reaches, by the handles every question uses, or with --json the engine's own report"
    )]
    Run(Run),
    #[command(
        about = "Check whether a program reaches an exact target configuration, as Prism does; exits 1 unless it is reached"
    )]
    Prism(Prism),
    #[command(
        about = "Check a program: diagnostics, then claims answered holds, fails or unknown, over every future or every plain schedule; exits 1 unless all hold"
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
    Inspect(Inspect),
    #[command(about = "Explain why a configuration, event or occurrence is here")]
    Cause(Cause),
    #[command(
        about = "Explain why not: the configurations nearest a target, or why a rule does not fire"
    )]
    Miss(Miss),
    #[command(about = "List the events that can happen at a configuration")]
    Step(Step),
    #[command(
        about = "Compare two programs by the configurations and events they reach; exits 1 unless both close and agree"
    )]
    Compare(Compare),
    #[command(
        about = "Describe a program up to the names of its atoms, or group programs by shape"
    )]
    Shape(Shape),
    #[command(
        about = "Serve Spectrum to agents over the Model Context Protocol on standard input and output, reading only files inside the directory it starts in"
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

fn text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

impl Source {
    // The program the files and inline source make; a command that gives neither names no program.
    pub fn subject(&self, file: &[PathBuf]) -> Result<Subject, Failure> {
        if file.is_empty() && self.source.is_none() {
            return Err(Failure::new(
                Code::Request,
                "name the program: give its files, or its text with --source",
            ));
        }
        Ok(Subject {
            file: file.iter().map(|path| text(path)).collect(),
            source: self.source.clone(),
            library: self.library.iter().map(|path| text(path)).collect(),
        })
    }
}

#[derive(Args)]
pub struct Budget {
    #[arg(long, default_value_t = spectrum::budget::Budget::default().work, help = "Work steps before the search stops")]
    pub work: usize,
    #[arg(
        long,
        help = "Configurations kept: 4,096 by default, and with --engine metal as many as the GPU holds"
    )]
    pub configuration: Option<usize>,
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

// How a question's recording explores its program: every future, every schedule of plain events,
// or one direct path.
#[derive(Args)]
pub struct Search {
    #[arg(
        long,
        help = "Follow one direct path instead of exploring every future; each step fires the first event within the limits that the scheduler finds"
    )]
    pub path: bool,
    #[arg(
        long,
        conflicts_with = "path",
        help = "Explore every schedule of plain events, those a configuration's own matches identify, without inference, on laser or metal"
    )]
    pub plain: bool,
    #[arg(
        long,
        conflicts_with = "path",
        requires_if("metal", "plain"),
        help = "The engine: laser, the default, or interpreter records every event of every future; with --plain, laser records every plain schedule, and metal explores them at GPU scale through the program's net of parts, keeping only counts, ends and cycles, so it answers explore and check alone"
    )]
    pub engine: Option<Engine>,
}

// The flags every question's recording takes beside its files, grouped so that each verb lists
// them alike and hands them on as one. --preserve completes an exact target or the goal, so it
// needs one of them: the goal here, or --exact where a verb takes exact targets.
#[derive(Args)]
#[command(group(ArgGroup::new("aim").multiple(true)))]
pub struct Recording {
    #[command(flatten)]
    pub source: Source,
    #[command(flatten)]
    pub budget: Budget,
    #[command(flatten)]
    pub search: Search,
    #[arg(
        long,
        requires = "path",
        group = "aim",
        help = "With --path, the complete configuration the path stops at"
    )]
    pub goal: Option<String>,
    #[arg(
        long,
        requires = "aim",
        help = "Exact targets and the goal also list every loaded root rule"
    )]
    pub preserve: bool,
}

#[derive(Args)]
pub struct Print {
    #[arg(long, help = "Print the answer as a JSON envelope")]
    pub json: bool,
}

// Each kind of claim with its flag and help, in the order the help lists them.
const KIND: [(Kind, &str, &str); 6] = [
    (
        Kind::Reach,
        "reach",
        "Claim that some configuration matches this pattern",
    ),
    (
        Kind::Avoid,
        "avoid",
        "Claim that no configuration matches this pattern",
    ),
    (
        Kind::Always,
        "always",
        "Claim that every configuration matches this pattern",
    ),
    (
        Kind::Inevitable,
        "inevitable",
        "Claim that every run reaches a configuration matching this pattern",
    ),
    (
        Kind::Outcome,
        "outcome",
        "Claim that every end configuration matches this pattern",
    ),
    (
        Kind::End,
        "end",
        "Claim that every run ends, and ends at a configuration matching this pattern",
    ),
];

// Each claim's kind and pattern in the order the command line gives them, which answers keep
// whatever the kinds.
pub struct Claim {
    pub pattern: Vec<(Kind, String)>,
    pub exact: bool,
}

impl FromArgMatches for Claim {
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, clap::Error> {
        let mut placed = KIND
            .iter()
            .flat_map(|&(kind, name, _)| {
                matches
                    .indices_of(name)
                    .into_iter()
                    .flatten()
                    .zip(matches.get_many::<String>(name).into_iter().flatten())
                    .map(move |(index, pattern)| (index, kind, pattern.clone()))
            })
            .collect::<Vec<_>>();
        placed.sort_by_key(|&(index, ..)| index);
        Ok(Self {
            pattern: placed
                .into_iter()
                .map(|(_, kind, pattern)| (kind, pattern))
                .collect(),
            exact: matches.get_flag("exact"),
        })
    }

    fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), clap::Error> {
        *self = Self::from_arg_matches(matches)?;
        Ok(())
    }
}

impl Args for Claim {
    fn augment_args(command: Command) -> Command {
        KIND.iter()
            .fold(command, |command, &(_, name, help)| {
                command.arg(
                    Arg::new(name)
                        .long(name)
                        .value_name("PATTERN")
                        .value_parser(clap::value_parser!(String))
                        .action(ArgAction::Append)
                        .help(help),
                )
            })
            .arg(
                Arg::new("exact")
                    .long("exact")
                    .action(ArgAction::SetTrue)
                    .group("aim")
                    .help("Read every claim's pattern as a complete configuration, as Prism does"),
            )
    }

    fn augment_args_for_update(command: Command) -> Command {
        Self::augment_args(command)
    }
}

#[derive(Args)]
pub struct Lower {
    #[arg(help = FILE)]
    pub file: Vec<PathBuf>,
    #[command(flatten)]
    pub source: Source,
}

// The cores the machine has, or one where it cannot tell.
fn core() -> NonZeroUsize {
    std::thread::available_parallelism().unwrap_or(NonZeroUsize::MIN)
}

// Run and prism list every configuration, which metal does not keep, so their help leaves it out.
#[derive(Args)]
#[command(mut_arg("configuration", |argument| argument.help("Configurations kept, 4,096 by default")))]
pub struct Run {
    #[arg(help = FILE)]
    pub file: Vec<PathBuf>,
    #[command(flatten)]
    pub source: Source,
    #[command(flatten)]
    pub budget: Budget,
    #[arg(
        long,
        requires = "json",
        default_value_t = core(),
        help = "With --json, threads that explore in parallel, every core by default; the text explores on every core"
    )]
    pub worker: NonZeroUsize,
    #[arg(
        long,
        help = "Explore every schedule of plain events, those a configuration's own matches identify, without inference, on laser"
    )]
    pub plain: bool,
    #[arg(
        long,
        help = "The engine that explores: laser, the default, which carries matches back along events and names configurations by their components, or interpreter; both close with the same configurations and handles, and plain mode runs on laser"
    )]
    pub engine: Option<Engine>,
    #[arg(
        long,
        help = "Print the engine's own report as JSON, its configurations numbered in the order the engine found them"
    )]
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
        help = "The target also lists every loaded root rule, as photonic_test's targets do"
    )]
    pub preserve: bool,
    #[arg(
        long,
        conflicts_with_all = ["engine", "plain"],
        help = "Follow one direct execution path; a target it does not reach stays unknown"
    )]
    pub path: bool,
}

#[derive(Args)]
pub struct Check {
    #[arg(help = FILE)]
    pub file: Vec<PathBuf>,
    #[command(flatten)]
    pub recording: Recording,
    #[command(flatten)]
    pub claim: Claim,
    #[command(flatten)]
    pub print: Print,
}

#[derive(Args)]
pub struct Explore {
    #[arg(help = FILE)]
    pub file: Vec<PathBuf>,
    #[command(flatten)]
    pub recording: Recording,
    #[arg(long, default_value_t = spectrum::explore::LIMIT, help = "End configurations listed")]
    pub limit: usize,
    #[command(flatten)]
    pub print: Print,
}

#[derive(Args)]
pub struct Select {
    #[arg(help = FILE)]
    pub file: Vec<PathBuf>,
    #[arg(
        long,
        help = "A Photonic pattern: B, B.X, B, C, ().([A] B), (K, [K] L) or [B, C] D"
    )]
    pub pattern: String,
    #[arg(long, default_value_t = spectrum::select::LIMIT, help = "Matches listed")]
    pub limit: usize,
    #[arg(long, default_value_t = 0, help = "Matches skipped")]
    pub offset: usize,
    #[command(flatten)]
    pub recording: Recording,
    #[command(flatten)]
    pub print: Print,
}

#[derive(Args)]
pub struct Inspect {
    #[arg(
        required = true,
        help = "Program files, then a handle such as r2, s11, e12, s11.c0, s11.o1 or s10.f1"
    )]
    pub argument: Vec<String>,
    #[command(flatten)]
    pub recording: Recording,
    #[command(flatten)]
    pub print: Print,
}

#[derive(Args)]
pub struct Cause {
    #[arg(
        required = true,
        help = "Program files, then a configuration, event or occurrence handle, such as s11, e12 or s11.o1"
    )]
    pub argument: Vec<String>,
    #[command(flatten)]
    pub recording: Recording,
    #[command(flatten)]
    pub print: Print,
}

#[derive(Args)]
pub struct Step {
    #[arg(
        help = "Program files, then optionally the configuration to step from, such as s11; s0, the start, by default"
    )]
    pub argument: Vec<String>,
    #[command(flatten)]
    pub recording: Recording,
    #[command(flatten)]
    pub print: Print,
}

#[derive(Args)]
pub struct Miss {
    #[arg(help = "Program files, then optionally a rule handle such as r3")]
    pub argument: Vec<String>,
    #[arg(
        long,
        help = "A pattern, or with --exact a complete configuration, to reach"
    )]
    pub target: Option<String>,
    #[arg(
        long,
        requires = "target",
        group = "aim",
        help = "Compare whole configurations as Prism does: coherences, live rules and open scopes"
    )]
    pub exact: bool,
    #[arg(long, default_value_t = spectrum::miss::LIMIT, help = "Configurations listed")]
    pub limit: usize,
    #[command(flatten)]
    pub recording: Recording,
    #[command(flatten)]
    pub print: Print,
}

// Compare explores both programs alike, so what shapes one recording shapes both.
#[derive(Args)]
#[command(
    mut_arg("source", |argument| argument.help("Photonic source added after the file of each program")),
    mut_arg("library", |argument| argument.help("Load a declaration-only library file into each program; repeat for each")),
    mut_arg("goal", |argument| argument.help("With --path, the complete configuration both paths stop at")),
)]
pub struct Compare {
    #[arg(help = "The program before")]
    pub left: PathBuf,
    #[arg(help = "The program after")]
    pub right: PathBuf,
    #[arg(long, default_value_t = spectrum::compare::LIMIT, help = "Differences listed on each side")]
    pub limit: usize,
    #[command(flatten)]
    pub recording: Recording,
    #[command(flatten)]
    pub claim: Claim,
    #[command(flatten)]
    pub print: Print,
}

#[derive(Args)]
#[command(
    mut_arg("source", |argument| argument.help("Photonic source added after the file of each program, or the one program when no file is given")),
    mut_arg("library", |argument| argument.help("Load a declaration-only library file into each program; repeat for each")),
)]
pub struct Shape {
    #[arg(help = "One program to describe, or several to group by shape")]
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
