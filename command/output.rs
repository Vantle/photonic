use miette::IntoDiagnostic;
use spectrum::failure::Failure;
use std::io::Write;
use std::process::ExitCode;

// Whether a write failed only because the reader closed its end, as head does once it has read
// enough. What was left unwritten is dropped, and the command still exits with its answer's code.
pub fn closed(error: &std::io::Error) -> bool {
    error.kind() == std::io::ErrorKind::BrokenPipe
}

fn quiet(result: std::io::Result<()>) -> miette::Result<()> {
    match result {
        Err(error) if closed(&error) => Ok(()),
        result => result.into_diagnostic(),
    }
}

pub fn print(text: &str) -> miette::Result<()> {
    let mut output = std::io::stdout().lock();
    quiet(writeln!(output, "{text}").and_then(|()| output.flush()))
}

pub fn write(value: &impl serde::Serialize, compact: bool) -> miette::Result<()> {
    let mut output = std::io::BufWriter::with_capacity(1 << 16, std::io::stdout().lock());
    let written = if compact {
        serde_json::to_writer(&mut output, value)
    } else {
        serde_json::to_writer_pretty(&mut output, value)
    };
    quiet(
        written
            .map_err(std::io::Error::from)
            .and_then(|()| writeln!(output))
            .and_then(|()| output.flush()),
    )
}

// A failure as the command prints it and the protocol server returns it: its code, then its text.
pub fn error(failure: &Failure) -> String {
    format!("error[{}]: {failure}", failure.code)
}

pub fn fail(failure: &Failure) -> miette::Result<ExitCode> {
    quiet(writeln!(std::io::stderr().lock(), "{}", error(failure)))?;
    Ok(ExitCode::FAILURE)
}
