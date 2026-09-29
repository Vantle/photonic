#![forbid(unsafe_code)]

mod argument;
mod engine;
mod output;
mod server;
mod verb;

use clap::Parser;
use miette::IntoDiagnostic;
use std::process::ExitCode;

use argument::{Argument, Operation};

fn main() -> miette::Result<ExitCode> {
    if let Some(directory) = std::env::var_os("BUILD_WORKING_DIRECTORY") {
        std::env::set_current_dir(directory).into_diagnostic()?;
    }
    match Argument::parse().operation {
        Operation::Lower(argument) => engine::lower(&argument),
        Operation::Run(argument) => engine::run(&argument),
        Operation::Prism(argument) => engine::prism(&argument),
        Operation::Check(argument) => verb::check(argument),
        Operation::Explore(argument) => verb::explore(argument),
        Operation::Select(argument) => verb::select(argument),
        Operation::Inspect(argument) => verb::inspect(argument),
        Operation::Cause(argument) => verb::cause(argument),
        Operation::Miss(argument) => verb::miss(argument),
        Operation::Step(argument) => verb::step(argument),
        Operation::Compare(argument) => verb::compare(argument),
        Operation::Shape(argument) => verb::shape(argument),
        Operation::Mcp => server::serve(),
    }
}
