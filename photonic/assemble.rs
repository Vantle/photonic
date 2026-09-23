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
    #[arg(long)]
    output: PathBuf,
}

fn main() -> miette::Result<()> {
    let argument = Argument::parse();
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
