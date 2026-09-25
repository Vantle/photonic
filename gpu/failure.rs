use thiserror::Error;

#[derive(Debug, Error)]
pub enum Failure {
    #[error("the GPU is unavailable: {0}")]
    Unavailable(String),
    #[error("could not allocate {length} bytes of GPU memory")]
    Allocation { length: usize },
    #[error("could not compile GPU kernels: {0}")]
    Compile(String),
    #[error("could not prepare a GPU kernel: {0}")]
    Kernel(String),
    #[error("GPU execution failed: {0}")]
    Execution(String),
    #[error("GPU training needs optimizer state before its first step")]
    Unprepared,
    #[error("the model holds {expected} parameters but {actual} were given")]
    Length { expected: usize, actual: usize },
}
