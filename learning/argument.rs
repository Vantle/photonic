use clap::{ArgGroup, Args, Parser, Subcommand, ValueEnum};
use learning::edit::Bound;
use learning::encoding::{DIMENSION, Shape};
use learning::export::BUDGET;
use learning::play;
use learning::pool::SYNTHETIC;
use learning::search;
use learning::solution::Budget;
use learning::task::{self, Goal};
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

fn processor(text: &str) -> Result<f64, String> {
    let value = text.parse::<f64>().map_err(|error| error.to_string())?;
    Goal::new(value, Goal::default().size())
        .map(|goal| goal.processor())
        .map_err(|failure| failure.to_string())
}

fn weight(text: &str) -> Result<f64, String> {
    let value = text.parse::<f64>().map_err(|error| error.to_string())?;
    Goal::new(Goal::default().processor(), value)
        .map(|goal| goal.size())
        .map_err(|failure| failure.to_string())
}

fn share(text: &str) -> Result<f64, String> {
    let value = text.parse::<f64>().map_err(|error| error.to_string())?;
    if !(0.0..=1.0).contains(&value) {
        return Err("a share must be a number from 0 to 1".to_owned());
    }
    Ok(value)
}

fn count(text: &str) -> Result<usize, String> {
    let value = text.parse::<usize>().map_err(|error| error.to_string())?;
    if value == 0 {
        return Err("the count must be at least 1".to_owned());
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
    #[command(about = "Train the network by self-play on the task pool")]
    Train(Train),
    #[command(about = "Import a program as a task and train with a share of the episodes on it")]
    Optimize(Optimize),
    #[command(about = "Report the archive, or one task's reference and best programs")]
    Status(Status),
    #[command(about = "Check archived programs against the runtime's kernel and Prism")]
    Verify(Verify),
    #[command(about = "Search for the cheapest programs of tasks, exhaustively and guided")]
    Solve(Solve),
    #[command(about = "Alternate generating new behaviors, solving them and training")]
    Improve(Improve),
    #[command(about = "Teach levels of increasing complexity, examined against proven optima")]
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

#[derive(Clone, Copy, Debug, Args)]
pub struct Objective {
    #[arg(
        long,
        value_parser = processor,
        default_value_t = Goal::default().processor(),
        help = "Processors in the Brent time bound of tasks without their own goal"
    )]
    pub processor: f64,
    #[arg(
        long,
        value_parser = weight,
        default_value_t = Goal::default().size(),
        help = "Weight of program size against time for tasks without their own goal"
    )]
    pub size: f64,
}

impl Objective {
    pub fn goal(&self) -> Result<Goal, task::Failure> {
        Goal::new(self.processor, self.size)
    }
}

#[derive(Clone, Debug, Args)]
pub struct Session {
    #[arg(long, value_enum, default_value_t = Device::Auto, help = "Where the network runs for self-play and training")]
    pub device: Device,
    #[arg(
        long,
        help = "Seconds to run, which also bounds every evaluation; runs until interrupted when omitted"
    )]
    pub duration: Option<u64>,
    #[arg(
        long,
        value_parser = count,
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
        value_parser = count,
        default_value_t = play::Setting::default().game,
        help = "Concurrent games per self-play thread"
    )]
    pub game: usize,
    #[arg(
        long,
        default_value_t = search::Setting::default().simulation,
        help = "Search simulations per decision"
    )]
    pub simulation: usize,
    #[arg(
        long,
        default_value_t = search::Setting::default().considered,
        help = "Root actions considered by sequential halving"
    )]
    pub considered: usize,
    #[arg(
        long,
        default_value_t = search::Setting::default().step,
        help = "Edits per episode"
    )]
    pub step: usize,
    #[command(flatten)]
    pub objective: Objective,
    #[arg(
        long,
        default_value_t = Bound::default().depth,
        help = "Deepest rule nesting the agent may create; a task whose reference nests deeper keeps its reference's depth"
    )]
    pub nesting: usize,
    #[arg(
        long,
        value_parser = share,
        default_value_t = play::Setting::default().focus,
        help = "Share of the episodes spent on the tasks the command focuses on"
    )]
    pub focus: f64,
    #[arg(
        long,
        value_parser = share,
        default_value_t = play::Setting::default().infer,
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
        default_value_t = play::Setting::default().race,
        help = "Race self-play games in pairs from the same start and train on each winner's decisions this many times; 0 plays alone"
    )]
    pub race: usize,
    #[arg(long, default_value_t = 30, help = "Seconds between progress reports")]
    pub report: u64,
    #[arg(long, default_value_t = 7, help = "Random seed")]
    pub seed: u64,
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
    #[arg(long, help = "Search without training or saving the network")]
    pub frozen: bool,
    #[arg(
        long,
        default_value_t = SYNTHETIC,
        help = "Synthetic tasks created for a new pool"
    )]
    pub synthetic: usize,
    #[arg(
        long,
        default_value_t = 8,
        help = "New synthetic tasks added to an existing pool on each run"
    )]
    pub fresh: usize,
    #[arg(long, help = "Tasks to focus on; repeat for several")]
    pub task: Vec<String>,
}

#[derive(Debug, Args)]
pub struct Optimize {
    #[command(flatten)]
    pub home: Home,
    #[command(flatten)]
    pub session: Session,
    #[arg(long, help = "Search without training or saving the network")]
    pub frozen: bool,
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
}

#[derive(Debug, Args)]
#[command(group(ArgGroup::new("measure").args(["processor", "size"]).multiple(true).requires("task")))]
pub struct Status {
    #[command(flatten)]
    pub home: Home,
    #[command(flatten)]
    pub objective: Objective,
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
    pub level: usize,
    #[arg(
        long,
        default_value_t = 20,
        help = "Held-out behaviors with proven optima in each level's exam"
    )]
    pub exam: usize,
    #[arg(
        long,
        default_value_t = 40,
        help = "New behaviors generated to train on in each level"
    )]
    pub fresh: usize,
    #[arg(long, default_value_t = 300, help = "Seconds of training per round")]
    pub practice: u64,
    #[arg(
        long,
        default_value_t = 5,
        help = "Seconds of exhaustive search to prove or find each new behavior's program"
    )]
    pub enumerate: u64,
    #[arg(
        long,
        default_value_t = 5,
        help = "Seconds of guided search per training behavior that exhaustive search leaves unproven or cannot take"
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
        value_parser = share,
        default_value_t = 0.9,
        help = "Share of a level's exam that must reach the proven optimum before the next level"
    )]
    pub mastery: f64,
    #[arg(
        long,
        value_parser = share,
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
    pub round: usize,
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
    pub practice: u64,
    #[arg(
        long,
        default_value_t = 5,
        help = "Seconds of exhaustive search per new behavior"
    )]
    pub enumerate: u64,
    #[arg(
        long,
        default_value_t = 10,
        help = "Seconds of guided search per new behavior that exhaustive search leaves unproven or cannot take"
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
    #[arg(
        long,
        default_value_t = Budget::default().size,
        help = "Largest program size to examine"
    )]
    pub limit: usize,
    #[arg(
        long,
        value_parser = processor,
        help = "Processors in the Brent time bound: sets the goal of the task that --input and --output define, or solves a copy of each --task task under it; the rest of the goal is the task's own or the default"
    )]
    pub processor: Option<f64>,
    #[arg(
        long,
        value_parser = weight,
        help = "Weight of program size against time, applied like --processor"
    )]
    pub size: Option<f64>,
    #[arg(
        long,
        help = "Seconds of exhaustive search per task, which also bounds every evaluation; unlimited when omitted"
    )]
    pub enumerate: Option<u64>,
    #[arg(
        long,
        help = "Ignore every known program and solve from the tests alone"
    )]
    pub blind: bool,
    #[arg(
        long,
        default_value_t = 0,
        help = "Seconds of search guided by the learned network for each task that exhaustive search leaves unproven or cannot take"
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
        default_value_t = BUDGET,
        help = "Prism work budget per example"
    )]
    pub budget: usize,
}
