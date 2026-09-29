use photonic::laser::net::Unsupported;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Failure {
    #[error(transparent)]
    Metal(#[from] metal::failure::Failure),
    #[error("the program has no net of parts: {0}")]
    Unsupported(#[from] Unsupported),
    #[error("the tables still lack a part after grounding it")]
    Missing,
    #[error("{count} successors of one pass are more than a pass can number")]
    Candidate { count: usize },
    #[error("{count} configurations are more than the table can number")]
    Configuration { count: usize },
    #[error("the arena's segments cannot hold {words} more words")]
    Arena { words: usize },
}
