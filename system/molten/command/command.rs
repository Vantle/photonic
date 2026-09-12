#![forbid(unsafe_code)]

mod argument;

use clap::Parser;
use miette::{IntoDiagnostic, NamedSource, WrapErr};

use argument::{Argument, Operation};

fn main() -> miette::Result<()> {
    let Argument { operation } = Argument::parse();
    let Operation::Parse { path } = operation;
    let source = std::fs::read_to_string(&path)
        .into_diagnostic()
        .wrap_err_with(|| format!("could not read {}", path.display()))?;
    match molten::parser::parse(&source) {
        Ok(tree) => println!("{:#?}", tree.node()),
        Err(failure) => {
            return Err(miette::Report::new(failure)
                .with_source_code(NamedSource::new(path.display().to_string(), source)));
        }
    }
    Ok(())
}
