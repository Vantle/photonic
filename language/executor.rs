use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rayon::{ThreadPool, ThreadPoolBuildError, ThreadPoolBuilder};
use std::num::NonZeroUsize;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("could not create executor: {0}")]
pub struct Failure(ThreadPoolBuildError);

pub struct Executor {
    pool: ThreadPool,
}

impl Executor {
    pub fn new(worker: NonZeroUsize) -> Result<Self, Failure> {
        let pool = ThreadPoolBuilder::new()
            .num_threads(worker.get())
            .build()
            .map_err(Failure)?;
        Ok(Self { pool })
    }

    pub(crate) fn concurrent(&self) -> bool {
        self.pool.current_num_threads() > 1
    }

    pub(crate) fn map<Input: Send, Output: Send>(
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
