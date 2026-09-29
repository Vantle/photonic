use miette::IntoDiagnostic;
use spectrum::failure::Failure;
use std::io::Write;
use std::process::ExitCode;

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

pub fn fail(failure: &Failure) -> miette::Result<ExitCode> {
    writeln!(
        std::io::stderr().lock(),
        "error[{}]: {failure}",
        failure.code
    )
    .into_diagnostic()?;
    Ok(ExitCode::FAILURE)
}
