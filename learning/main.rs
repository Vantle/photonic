#![forbid(unsafe_code)]

mod argument;
mod course;
mod improve;
mod inspect;
mod output;
mod practice;
mod setup;
mod solve;

use argument::{Argument, Command};
use clap::Parser;

fn main() -> miette::Result<()> {
    match Argument::parse().command {
        Command::Train(argument) => practice::train(argument),
        Command::Optimize(argument) => practice::optimize(argument),
        Command::Status(argument) => inspect::status(&argument),
        Command::Verify(argument) => inspect::check(&argument),
        Command::Solve(argument) => solve::run(&argument),
        Command::Improve(argument) => improve::run(argument),
        Command::Curriculum(argument) => course::run(argument),
    }
}
