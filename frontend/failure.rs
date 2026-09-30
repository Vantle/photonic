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
    #[error("an input cannot be a scope; match a rule without parentheses, as in [[A] B]")]
    #[diagnostic(code(photonic::input))]
    Input {
        #[label("this input is a scope")]
        span: SourceSpan,
    },
    #[error("Photonic expansion exceeds its {limit}-unit frontend budget")]
    #[diagnostic(code(photonic::expansion))]
    Expansion {
        limit: usize,
        #[label("expanding this exceeds the frontend budget")]
        span: SourceSpan,
    },
    #[error("this library lists a coherence or scope; a library holds only rules")]
    #[diagnostic(code(photonic::library))]
    Library {
        #[label("a library holds only rules")]
        span: SourceSpan,
    },
    #[error("invalid Photonic program: {message}")]
    #[diagnostic(code(photonic::json))]
    Json {
        message: String,
        #[label("{message}")]
        span: SourceSpan,
    },
    #[error("could not read {path}")]
    #[diagnostic(code(photonic::read))]
    Read {
        path: String,
        #[source]
        error: std::io::Error,
    },
    #[error("{path} is not UTF-8 text; save it as UTF-8")]
    #[diagnostic(code(photonic::encoding))]
    Encoding { path: String },
}
