use clap::Parser;
use photonic::runtime::Limit;
use std::num::NonZeroUsize;

mod evaluation;

#[derive(Parser)]
struct Argument {
    #[arg(long, default_value = "4096")]
    width: NonZeroUsize,
    #[arg(long, default_value = "1000")]
    length: NonZeroUsize,
    #[arg(long, default_value = "9")]
    sample: NonZeroUsize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argument = Argument::parse();
    let content = vec!["A.B"; argument.width.get()].join(",");
    let mut source = format!("{content},Stage0.A.C,\n[A.B,C] Never,\n");
    for stage in 0..argument.length.get() {
        source.push_str(&format!("[Stage{stage}] Stage{},\n", stage + 1));
    }
    let program = frontend::lowering::parse(&source)?;
    let mut target =
        frontend::lowering::parse(&format!("{content},Stage{}.A.C", argument.length.get()))?;
    target.preserve(&program);
    let limit = Limit {
        configuration: argument.length.get() + 1,
        record: 100_000_000,
        coherence: argument.width.get() + 1,
        occurrence: argument.width.get() * 2 + 3,
        scope: 1,
    };
    evaluation::warm(&program, &target, Some(limit))?;
    let measurement = (0..argument.sample.get())
        .map(|_| evaluation::evaluate(program.clone(), target.clone(), Some(limit)))
        .collect::<Result<Vec<_>, _>>()?;
    serde_json::to_writer_pretty(
        std::io::stdout().lock(),
        &serde_json::json!({
            "width": argument.width, "length": argument.length, "measurement": measurement,
        }),
    )?;
    Ok(())
}
