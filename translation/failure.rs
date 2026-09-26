use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Diagnostic, Error, Eq, PartialEq)]
pub enum Failure {
    #[error("a program names at most {limit} atoms")]
    #[diagnostic(code(translation::vocabulary))]
    Vocabulary { limit: usize },
    #[error("the vocabulary names {name} twice")]
    #[diagnostic(code(translation::duplicate))]
    Duplicate { name: String },
    #[error("{name:?} is not an atom name")]
    #[diagnostic(code(translation::name))]
    Name { name: String },
}
