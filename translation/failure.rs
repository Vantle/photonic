use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Diagnostic, Error, Eq, PartialEq)]
pub enum Failure {
    #[error("a program names at most {limit} atoms")]
    #[diagnostic(code(translation::vocabulary))]
    Vocabulary { limit: usize },
    #[error("the atom {name} is outside the vocabulary")]
    #[diagnostic(code(translation::unknown))]
    Unknown { name: String },
    #[error(
        "{name} shares its term with other brackets; the code model holds one bracket per rule"
    )]
    #[diagnostic(code(translation::rest))]
    Rest { name: String },
}
