use clap::Parser;
use frontend::source::Program;
use miette::IntoDiagnostic;
use std::path::PathBuf;

#[derive(Parser)]
struct Argument {
    #[arg(long)]
    source: Vec<PathBuf>,
    #[arg(long)]
    library: Vec<PathBuf>,
    #[arg(
        long,
        help = "A file that is only parsed, such as a test's source or target, so an error fails the build"
    )]
    check: Vec<PathBuf>,
    #[arg(long)]
    output: PathBuf,
}

fn main() -> miette::Result<()> {
    let argument = Argument::parse();
    for path in &argument.check {
        frontend::lowering::read(path)?;
    }
    let mut program = Program::default();
    for path in &argument.library {
        program.declare(frontend::lowering::read(path)?, path.display())?;
    }
    for path in &argument.source {
        program.append(frontend::lowering::read(path)?);
    }
    let encoded = serde_json::to_vec(&program).into_diagnostic()?;
    std::fs::write(argument.output, encoded).into_diagnostic()
}
