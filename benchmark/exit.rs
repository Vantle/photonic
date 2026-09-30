use std::process::{ExitCode, Termination};

// A failure prints its message, where returning it from main would print its debug form.
pub fn code(result: Result<impl Termination, Box<dyn std::error::Error>>) -> ExitCode {
    match result {
        Ok(value) => value.report(),
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
