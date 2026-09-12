use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Debug, Diagnostic, Error)]
pub enum Failure {
    #[error("invalid Photonic syntax: {message}")]
    #[diagnostic(code(photonic::syntax))]
    Syntax {
        message: String,
        #[label("{message}")]
        span: SourceSpan,
    },
    #[error("syntax nesting exceeds {limit} levels")]
    #[diagnostic(code(photonic::depth))]
    Depth {
        limit: usize,
        #[label("nesting limit exceeded here")]
        span: SourceSpan,
    },
}
