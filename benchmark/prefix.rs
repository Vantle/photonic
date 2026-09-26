use clap::Parser;
use photonic::runtime::Limit;
use std::num::NonZeroUsize;

mod evaluation;

#[derive(Parser)]
struct Argument {
    #[arg(long, default_value = "8")]
    width: NonZeroUsize,
    #[arg(long, default_value = "100")]
    length: NonZeroUsize,
    #[arg(long, default_value = "7")]
    sample: NonZeroUsize,
    #[arg(long)]
    changing: bool,
    #[arg(long, default_value = "32")]
    delay: NonZeroUsize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argument = Argument::parse();
    let particle = ["A"; 8].join(".");
    let content = vec!["B.C.E"; argument.width.get()].join(",");
    let pattern = vec!["B"; argument.width.get() - 1]
        .into_iter()
        .chain(["B.E.E"])
        .collect::<Vec<_>>()
        .join(",");
    let delay = vec!["D"; argument.delay.get()].join(",");
    let world = |stage| {
        if argument.changing {
            format!("Stage{stage}.{particle},{content},C,{delay}")
        } else {
            format!("{particle},{content},Stage{stage}.C,{delay}")
        }
    };
    let mut source = format!("{},\n[{particle},{pattern},C] Never,\n", world(0));
    for stage in 0..argument.length.get() {
        let suffix = if argument.changing {
            particle.as_str()
        } else {
            "C"
        };
        source.push_str(&format!(
            "[Stage{stage}.{suffix},{delay}] (Stage{}.{suffix},{delay}),\n",
            stage + 1
        ));
    }
    let program = frontend::lowering::parse(&source)?;
    let mut target = frontend::lowering::parse(&world(argument.length.get()))?;
    target.preserve(&program);
    let limit = Limit {
        configuration: argument.length.get() * 2 + 2,
        record: 100_000_000,
        coherence: argument.width.get() + argument.delay.get() + 2,
        occurrence: argument.width.get() * 3 + argument.delay.get() + 10,
        scope: 1,
    };
    evaluation::warm(&program, &target, Some(limit))?;
    let measurement = (0..argument.sample.get())
        .map(|_| evaluation::evaluate(program.clone(), target.clone(), Some(limit)))
        .collect::<Result<Vec<_>, _>>()?;
    serde_json::to_writer_pretty(
        std::io::stdout().lock(),
        &serde_json::json!({
            "width": argument.width, "length": argument.length,
            "changing": argument.changing, "delay": argument.delay, "measurement": measurement,
        }),
    )?;
    Ok(())
}
