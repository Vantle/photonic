use clap::Parser;
use photonic::runtime::Limit;
use std::num::NonZeroUsize;

mod evaluation;

#[derive(Parser)]
struct Argument {
    #[arg(long, default_value = "256")]
    width: NonZeroUsize,
    #[arg(long, default_value = "32")]
    term: NonZeroUsize,
    #[arg(long, default_value = "32")]
    count: NonZeroUsize,
    #[arg(long, default_value = "128")]
    length: NonZeroUsize,
    #[arg(long)]
    activation: bool,
    #[arg(long, default_value = "9")]
    sample: NonZeroUsize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argument = Argument::parse();
    let particle = (0..argument.term.get())
        .map(|position| format!("A{position}"))
        .collect::<Vec<_>>()
        .join(".");
    let content = vec![particle.as_str(); argument.width.get()].join(",");
    let gate = (0..argument.count.get())
        .map(|position| format!("G{position}"))
        .collect::<Vec<_>>()
        .join(".");
    let initial = if argument.activation {
        String::new()
    } else {
        format!(",{gate}")
    };
    let mut source = format!("{content},Stage0.{particle}{initial},\n");
    for position in 0..argument.count.get() {
        source.push_str(&format!("[{particle},G{position}.G{position}] Never,\n"));
    }
    for stage in 0..argument.length.get() {
        let activation = if argument.activation {
            (0..argument.count.get())
                .filter(|position| position % argument.length.get() == stage)
                .map(|position| format!(".G{position}"))
                .collect::<String>()
        } else {
            String::new()
        };
        source.push_str(&format!("[Stage{stage}] Stage{}{activation},\n", stage + 1));
    }
    let program = photonic::lowering::parse(&source)?;
    let separator = if argument.activation { "." } else { "," };
    let target = photonic::lowering::parse(&format!(
        "{content},Stage{}.{particle}{separator}{gate}",
        argument.length.get()
    ))?;
    let target = photonic::source::Program {
        rule: program.rule.clone(),
        ..target
    };
    let limit = Limit {
        state: argument.length.get() + 1,
        record: 100_000_000,
        world: argument.width.get() + 2,
        cell: (argument.width.get() + 1) * argument.term.get() + argument.count.get() + 1,
        frame: 1,
    };
    evaluation::warm(&program, &target, Some(limit));
    let measurement = (0..argument.sample.get())
        .map(|_| evaluation::evaluate(program.clone(), target.clone(), Some(limit)))
        .collect::<Vec<_>>();
    serde_json::to_writer_pretty(
        std::io::stdout().lock(),
        &serde_json::json!({
            "width": argument.width, "term": argument.term, "count": argument.count, "length": argument.length, "activation": argument.activation, "measurement": measurement,
        }),
    )?;
    Ok(())
}
