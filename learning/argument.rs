use clap::{Args, Parser, Subcommand, ValueEnum};
use learning::encoding::{DIMENSION, Shape};
use std::path::PathBuf;

fn width(text: &str) -> Result<usize, String> {
    let value = text.parse::<usize>().map_err(|error| error.to_string())?;
    if value == 0 || !value.is_multiple_of(DIMENSION) {
        return Err(format!(
            "the width must be a positive multiple of {DIMENSION}"
        ));
    }
    Ok(value)
}

#[derive(Debug, Parser)]
#[command(
    name = "learning",
    about = "Learn to optimize Photonic programs with Gumbel planning over program edits"
)]
pub struct Argument {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Train(Train),
    Optimize(Optimize),
    Status(Status),
    Verify(Verify),
    Solve(Solve),
    Improve(Improve),
    Curriculum(Course),
}

#[derive(Clone, Debug, Args)]
pub struct Home {
    #[arg(
        long,
        help = "Learner directory; defaults to ~/.cache/photonic/learning"
    )]
    pub home: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Device {
    Auto,
    Gpu,
    Cpu,
}

#[derive(Clone, Debug, Args)]
pub struct Session {
    #[arg(long, value_enum, default_value_t = Device::Auto, help = "Where the network runs for self-play and training")]
    pub device: Device,
    #[arg(long, help = "Seconds to run; runs until interrupted when omitted")]
    pub duration: Option<u64>,
    #[arg(
        long,
        help = "Self-play threads; defaults to the cores left after training, doubled while the GPU serves inference"
    )]
    pub worker: Option<usize>,
    #[arg(
        long,
        default_value_t = 4,
        help = "Training threads when training runs on the CPU"
    )]
    pub trainer: usize,
    #[arg(
        long,
        default_value_t = 8,
        help = "Concurrent games per self-play thread"
    )]
    pub game: usize,
    #[arg(long, default_value_t = 16, help = "Search simulations per decision")]
    pub simulation: usize,
    #[arg(
        long,
        default_value_t = 16,
        help = "Root actions considered by sequential halving"
    )]
    pub considered: usize,
    #[arg(long, default_value_t = 24, help = "Edits per episode")]
    pub step: usize,
    #[arg(
        long,
        default_value_t = 4.0,
        help = "Processors in the Brent time bound"
    )]
    pub processor: f64,
    #[arg(
        long,
        default_value_t = 0.05,
        help = "Weight of program size against time"
    )]
    pub size: f64,
    #[arg(
        long,
        default_value_t = 0,
        help = "Rule nesting depth the agent may create beyond the reference's own"
    )]
    pub nesting: usize,
    #[arg(
        long,
        default_value_t = 0.05,
        help = "Largest measured share of skipped candidates that proved no worse than their parent before the learned judge may skip exact checks; 0 checks every candidate"
    )]
    pub infer: f64,
    #[arg(
        long,
        default_value_t = 60,
        help = "Seconds between lessons that replay every best known program's construction to the network; 0 turns lessons off"
    )]
    pub teach: u64,
    #[arg(
        long,
        default_value_t = 0,
        help = "Race self-play games in pairs from the same start and train on each winner's decisions this many times; 0 plays alone"
    )]
    pub race: usize,
    #[arg(long, default_value_t = 30, help = "Seconds between progress reports")]
    pub report: u64,
    #[arg(long, default_value_t = 7, help = "Random seed")]
    pub seed: u64,
    #[arg(long, help = "Search without training or saving the network")]
    pub frozen: bool,
    #[arg(
        long,
        value_parser = width,
        default_value_t = Shape::default().width,
        help = "Network width; a saved narrower network grows to it"
    )]
    pub width: usize,
    #[arg(
        long,
        default_value_t = Shape::default().depth,
        help = "Transformer blocks; a saved shallower network grows to it"
    )]
    pub depth: usize,
    #[arg(
        long,
        default_value_t = Shape::default().hidden,
        help = "Feed-forward width; a saved narrower network grows to it"
    )]
    pub hidden: usize,
}

#[derive(Debug, Args)]
pub struct Train {
    #[command(flatten)]
    pub home: Home,
    #[command(flatten)]
    pub session: Session,
    #[arg(
        long,
        default_value_t = 48,
        help = "Synthetic tasks created for a new pool"
    )]
    pub synthetic: usize,
    #[arg(
        long,
        default_value_t = 8,
        help = "Synthetic tasks added to an existing pool on each run"
    )]
    pub grow: usize,
    #[arg(
        long,
        help = "Tasks that share half of the episodes; repeat for several"
    )]
    pub task: Vec<String>,
}

#[derive(Debug, Args)]
pub struct Optimize {
    #[command(flatten)]
    pub home: Home,
    #[command(flatten)]
    pub session: Session,
    #[arg(
        long,
        required = true,
        help = "Photonic sources holding the program's rules"
    )]
    pub program: Vec<PathBuf>,
    #[arg(long, help = "Photonic sources holding one input configuration each")]
    pub input: Vec<PathBuf>,
    #[arg(long, help = "Task name; defaults to the first program's file stem")]
    pub name: Option<String>,
    #[arg(
        long,
        default_value_t = 0.5,
        help = "Share of episodes spent on this program"
    )]
    pub focus: f64,
}

#[derive(Debug, Args)]
pub struct Status {
    #[command(flatten)]
    pub home: Home,
    #[arg(
        long,
        help = "Show one task's reference and best programs with their measurements"
    )]
    pub task: Option<String>,
}

#[derive(Debug, Args)]
pub struct Course {
    #[command(flatten)]
    pub home: Home,
    #[command(flatten)]
    pub session: Session,
    #[arg(
        long,
        default_value_t = 8,
        help = "Highest level to reach; level L holds behaviors of L rules"
    )]
    pub levels: usize,
    #[arg(
        long,
        default_value_t = 20,
        help = "Held-out behaviors with proven optima in each level's exam"
    )]
    pub exam: usize,
    #[arg(long, default_value_t = 40, help = "Training behaviors per level")]
    pub train: usize,
    #[arg(long, default_value_t = 300, help = "Seconds of training per round")]
    pub round: u64,
    #[arg(
        long,
        default_value_t = 5,
        help = "Seconds of exhaustive search to prove or find each new behavior's program"
    )]
    pub grade: u64,
    #[arg(
        long,
        default_value_t = 5,
        help = "Seconds of guided search per training behavior left unproven"
    )]
    pub guide: u64,
    #[arg(
        long,
        default_value_t = 256,
        help = "Programs guided search may examine per exam behavior"
    )]
    pub expansion: u64,
    #[arg(
        long,
        default_value_t = 0.9,
        help = "Share of a level's exam that must reach the proven optimum before the next level"
    )]
    pub mastery: f64,
    #[arg(
        long,
        default_value_t = 0.25,
        help = "Share of each round's focus drawn from earlier levels"
    )]
    pub rehearse: f64,
}

#[derive(Debug, Args)]
pub struct Improve {
    #[command(flatten)]
    pub home: Home,
    #[command(flatten)]
    pub session: Session,
    #[arg(
        long,
        default_value_t = 4,
        help = "Rounds of generating, solving and training"
    )]
    pub rounds: usize,
    #[arg(
        long,
        default_value_t = 50,
        help = "New behaviors defined by tests alone in each round"
    )]
    pub fresh: usize,
    #[arg(
        long,
        default_value_t = 600,
        help = "Seconds of training with lessons in each round"
    )]
    pub round: u64,
    #[arg(
        long,
        default_value_t = 5,
        help = "Seconds of exhaustive search per new behavior"
    )]
    pub budget: u64,
    #[arg(
        long,
        default_value_t = 10,
        help = "Seconds of guided search per new behavior that exhaustive search leaves unproven"
    )]
    pub guide: u64,
}

#[derive(Debug, Args)]
pub struct Solve {
    #[command(flatten)]
    pub home: Home,
    #[arg(
        long,
        help = "Tasks to solve; repeat for several; every task in the pool when omitted"
    )]
    pub task: Vec<String>,
    #[arg(
        long,
        help = "Photonic source holding one test's input coherences; pair each with an --output"
    )]
    pub input: Vec<PathBuf>,
    #[arg(
        long,
        help = "Photonic source holding the expected result of the --input in the same position"
    )]
    pub output: Vec<PathBuf>,
    #[arg(long, help = "Name of the task that --input and --output define")]
    pub name: Option<String>,
    #[arg(long, default_value_t = 16, help = "Largest program size to examine")]
    pub limit: usize,
    #[arg(
        long,
        help = "Processors in the time bound of the task that --input and --output define"
    )]
    pub processor: Option<f64>,
    #[arg(
        long,
        help = "Weight of program size against time for the task that --input and --output define"
    )]
    pub size: Option<f64>,
    #[arg(long, help = "Seconds allowed per task; unlimited when omitted")]
    pub budget: Option<u64>,
    #[arg(
        long,
        help = "Ignore every known program and solve from the tests alone"
    )]
    pub blind: bool,
    #[arg(
        long,
        default_value_t = 0,
        help = "Seconds of search guided by the learned network for each task that exhaustive search leaves unproven"
    )]
    pub guide: u64,
    #[arg(
        long,
        default_value_t = 1,
        help = "Cheapest programs to print per task"
    )]
    pub show: usize,
}

#[derive(Debug, Args)]
pub struct Verify {
    #[command(flatten)]
    pub home: Home,
    #[arg(long, help = "Verify one task; verifies every discovery when omitted")]
    pub task: Option<String>,
    #[arg(
        long,
        default_value_t = 2_000_000,
        help = "Prism work budget per example"
    )]
    pub budget: usize,
}
