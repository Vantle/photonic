use clap::Parser;
use photonic::source::Program;
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

fn read(path: &PathBuf) -> Result<Program, Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(path)?;
    photonic::lowering::parse(&source)
        .map_err(|failure| format!("{}: {failure}", path.display()).into())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argument = Argument::parse();
    let mut program = Program::default();
    for path in &argument.library {
        let library = read(path)?;
        if !library.initial.is_empty() {
            return Err(format!(
                "{}: a library must contain declarations only",
                path.display()
            )
            .into());
        }
        program.rule.extend(library.rule);
    }
    for path in &argument.source {
        let source = read(path)?;
        program.initial.extend(source.initial);
        program.rule.extend(source.rule);
    }
    std::fs::write(argument.output, serde_json::to_vec(&program)?)?;
    Ok(())
}
