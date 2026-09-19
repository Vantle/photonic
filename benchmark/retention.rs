use clap::Parser;

mod evaluation;

#[derive(Parser)]
struct Argument {
    #[arg(long, default_value_t = 200)]
    width: usize,
    #[arg(long, default_value_t = 1000)]
    length: usize,
    #[arg(long, default_value_t = 5)]
    sample: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argument = Argument::parse();
    let mut source = String::from("A,Stage.0\n");
    for index in 0..argument.width {
        source.push_str(&format!("[A,A] Never.{index}\n"));
    }
    for index in 0..argument.length {
        source.push_str(&format!("[Stage.{index}] Stage.{}\n", index + 1));
    }
    let program = photonic::lowering::parse(&source)?;
    let target = photonic::lowering::parse(&format!("A,Stage.{}", argument.length))?;
    evaluation::evaluate(program.clone(), target.clone());
    let measurement = (0..argument.sample)
        .map(|_| evaluation::evaluate(program.clone(), target.clone()))
        .collect::<Vec<_>>();
    serde_json::to_writer_pretty(
        std::io::stdout().lock(),
        &serde_json::json!({
            "width": argument.width, "length": argument.length, "measurement": measurement,
        }),
    )?;
    Ok(())
}
