use clap::Parser;
use std::num::NonZeroUsize;

mod evaluation;

#[derive(Parser)]
struct Argument {
    #[arg(long, default_value = "1024")]
    width: NonZeroUsize,
    #[arg(long, default_value_t = 1000)]
    length: usize,
    #[arg(long, default_value = "5")]
    sample: NonZeroUsize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argument = Argument::parse();
    let particle = vec!["A"; argument.width.get()].join(".");
    let mut source = format!("X.{particle},Stage0,\n[{particle}.A] Never,\n");
    for index in 0..argument.length {
        source.push_str(&format!("[Stage{index}] Stage{},\n", index + 1));
    }
    let program = photonic::lowering::parse(&source)?;
    let target = photonic::lowering::parse(&format!("X.{particle},Stage{}", argument.length))?;
    let target = photonic::source::Program {
        rule: program.rule.clone(),
        ..target
    };
    evaluation::warm(&program, &target, None);
    let measurement = (0..argument.sample.get())
        .map(|_| evaluation::evaluate(program.clone(), target.clone(), None))
        .collect::<Vec<_>>();
    serde_json::to_writer_pretty(
        std::io::stdout().lock(),
        &serde_json::json!({
            "width": argument.width, "length": argument.length, "measurement": measurement,
        }),
    )?;
    Ok(())
}
