use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Debug, Diagnostic, Error)]
pub enum Failure {
    #[error("invalid Photonic syntax: {message}")]
    #[diagnostic(code(frontend::syntax))]
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
    #[error("invalid Photonic expression: {message}")]
    #[diagnostic(code(frontend::lowering))]
    Lowering {
        message: String,
        #[label("{message}")]
        span: SourceSpan,
    },
    #[error("Photonic expansion exceeds its {limit}-unit frontend budget")]
    #[diagnostic(code(photonic::expansion))]
    Expansion {
        limit: usize,
        #[label("expanding this exceeds the frontend budget")]
        span: SourceSpan,
    },
    #[error("library {library} contains initial coherences or scopes; supply declarations only")]
    #[diagnostic(code(photonic::library))]
    Library { library: String },
    #[error("could not read {path}")]
    #[diagnostic(code(photonic::read))]
    Read {
        path: String,
        #[source]
        error: std::io::Error,
    },
}
