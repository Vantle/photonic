mod argument;
mod search;

use clap::Parser;
use frontend::lowering::parse;
use photonic::path::Search;
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
    let start = Instant::now();
    let program = parse(&source)?;
    println!(
        "{} ordinary rules; {} source bytes",
        program.rule.len(),
        source.len()
    );
    let mut target = parse(&target)?;
    target.preserve(&program);
    if let Some(directory) = &argument.directory {
        std::fs::create_dir_all(directory)?;
        std::fs::write(directory.join("program.wave"), &source)?;
        std::fs::write(
            directory.join("target.json"),
            serde_json::to_vec_pretty(&target)?,
        )?;
    }
    let mut search = Search::new(program, Some(target));
    search.run(argument.work, search::LIMIT);
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
