use miette::IntoDiagnostic;
use std::io::Write;

pub fn write(value: &impl serde::Serialize, compact: bool) -> miette::Result<()> {
    let mut output = std::io::BufWriter::with_capacity(1 << 16, std::io::stdout().lock());
    if compact {
        serde_json::to_writer(&mut output, value).into_diagnostic()?;
    } else {
        serde_json::to_writer_pretty(&mut output, value).into_diagnostic()?;
    }
    writeln!(output).into_diagnostic()?;
    output.flush().into_diagnostic()
}
