use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rayon::{ThreadPool, ThreadPoolBuildError, ThreadPoolBuilder};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Failure {
    #[error("executor requires at least one worker")]
    Zero,
    #[error("could not create executor: {0}")]
    Build(#[from] ThreadPoolBuildError),
}

pub struct Executor {
    pool: ThreadPool,
}

impl Executor {
    pub fn new(worker: usize) -> Result<Self, Failure> {
        if worker == 0 {
            return Err(Failure::Zero);
        }
        Ok(Self {
            pool: ThreadPoolBuilder::new().num_threads(worker).build()?,
        })
    }

    pub(crate) fn concurrent(&self) -> bool {
        self.pool.current_num_threads() > 1
    }

    pub fn map<Input: Send, Output: Send>(
        &self,
        input: Vec<Input>,
        operation: impl Fn(Input) -> Output + Sync + Send,
    ) -> Vec<Output> {
        if input.len() < 2 || self.pool.current_num_threads() == 1 {
            return input.into_iter().map(operation).collect();
        }
        self.pool
            .install(|| input.into_par_iter().map(operation).collect())
    }
}
