mod argument;

use clap::Parser;
use photonic::lowering::parse;
use photonic::path::Search;
use photonic::runtime::Limit;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argument = argument::Argument::parse();
    let (source, target) = argument.prepare()?;
    println!(
        "Claim: {} {} {} = {}; {}-trit operands",
        argument.left,
        argument.operation.symbol(),
        argument.right,
        argument.expected,
        argument.width()?
    );
    if matches!(argument.operation, argument::Operation::Divide) {
        println!(
            "Proposed remainder: {}; undefined: {}",
            argument.remainder, argument.undefined
        );
    }
    if let Some(directory) = &argument.directory {
        std::fs::create_dir_all(directory)?;
        std::fs::write(directory.join("program.wave"), &source)?;
        std::fs::write(directory.join("target.particle"), &target)?;
    }
    let start = Instant::now();
    let program = parse(&source)?;
    println!(
        "{} ordinary rules; {} source bytes",
        program.rule.len(),
        source.len()
    );
    let mut search = Search::new(program, parse(&target)?)?;
    search.run(
        argument.step,
        Limit {
            state: 4096,
            record: 1_000_000,
            cell: 4096,
            frame: 10,
            world: 4,
        },
    );
    let report = search.report();
    println!(
        "{:?}: {} events; {} work steps; {} ms",
        report.outcome,
        report.event.len(),
        report.work,
        start.elapsed().as_millis()
    );
    println!("The proposed result is a proof target; this runner does not calculate the answer.");
    Ok(())
}
