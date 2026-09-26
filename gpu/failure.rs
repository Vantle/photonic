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
    #[error("input {sample} is malformed: {defect}")]
    Input { sample: usize, defect: Defect },
}

#[derive(Debug, Error, PartialEq)]
pub enum Defect {
    #[error("it holds no token")]
    Empty,
    #[error("its {length} features do not divide into tokens of {field} fields")]
    Length { length: usize, field: usize },
    #[error("token {token} holds {value} in field {field}, which has {cardinality} values")]
    Value {
        token: usize,
        field: usize,
        value: u16,
        cardinality: usize,
    },
    #[error("a pointer names head {head} of {count}")]
    Head { head: u16, count: usize },
    #[error("a pointer names token {token} of {length}")]
    Token { token: u32, length: usize },
}
