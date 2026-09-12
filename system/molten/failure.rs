use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Debug, Diagnostic, Error)]
pub enum Failure {
    #[error("invalid Molten syntax: {message}")]
    #[diagnostic(code(molten::syntax))]
    Syntax {
        message: String,
        #[label("{message}")]
        span: SourceSpan,
    },
    #[error("syntax nesting exceeds {limit} levels")]
    #[diagnostic(code(molten::depth))]
    Depth {
        limit: usize,
        #[label("nesting limit exceeded here")]
        span: SourceSpan,
    },
}
