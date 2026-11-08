use rayon::iter::IndexedParallelIterator;
use rayon::prelude::{
    IntoParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator,
    ParallelSliceMut,
};
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::num::NonZeroUsize;
use std::sync::Arc;

/// Game-facing parallel job operations backed by a Nord worker pool.
#[derive(Clone)]
pub struct JobSystem {
    pool: Arc<ThreadPool>,
}

/// Worker-pool configuration owned by this job system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobSystemConfig {
    worker_threads: NonZeroUsize,
}

impl JobSystemConfig {
    /// Create a configuration with an explicit worker count.
    #[must_use]
    pub const fn new(worker_threads: NonZeroUsize) -> Self {
        Self { worker_threads }
    }

    /// Number of workers used by this job system.
    #[must_use]
    pub const fn worker_threads(self) -> NonZeroUsize {
        self.worker_threads
    }
}

impl Default for JobSystemConfig {
    fn default() -> Self {
        let available = std::thread::available_parallelism().unwrap_or(NonZeroUsize::MIN).get();
        Self {
            worker_threads: NonZeroUsize::new(available.saturating_sub(1).max(1))
                .unwrap_or(NonZeroUsize::MIN),
        }
    }
}

impl JobSystem {
    /// Create a job system with its own worker pool.
    ///
    /// # Errors
    ///
    /// Returns a Rayon build error if the worker pool cannot be created.
    pub fn new(config: JobSystemConfig) -> Result<Self, rayon::ThreadPoolBuildError> {
        let pool = ThreadPoolBuilder::new().num_threads(config.worker_threads.get()).build()?;
        Ok(Self { pool: Arc::new(pool) })
    }

    /// Number of dedicated workers available to scheduled jobs.
    #[must_use]
    pub fn worker_threads(&self) -> NonZeroUsize {
        NonZeroUsize::new(self.pool.current_num_threads()).unwrap_or(NonZeroUsize::MIN)
    }

    /// Run a read-only operation over a slice in parallel.
    ///
    /// Execution order is unspecified. The operation should be independent
    /// for each item and avoid blocking on work submitted to this same pool.
    pub fn for_each<T, Operation>(&self, items: &[T], operation: Operation)
    where
        T: Sync,
        Operation: Fn(&T) + Send + Sync,
    {
        let () = self.pool.install(|| items.par_iter().for_each(operation));
    }

    /// Run a disjoint mutable operation over a slice in parallel.
    ///
    /// Each item is exclusively borrowed by one invocation. Execution order
    /// is unspecified; shared mutable state still requires explicit sync.
    pub fn for_each_mut<T, Operation>(&self, items: &mut [T], operation: Operation)
    where
        T: Send,
        Operation: Fn(&mut T) + Send + Sync,
    {
        let () = self.pool.install(|| items.par_iter_mut().for_each(operation));
    }

    /// Map a slice in parallel, preserving the input order in the result.
    #[must_use]
    pub fn map<T, Output, Operation>(&self, items: &[T], operation: Operation) -> Vec<Output>
    where
        T: Sync,
        Output: Send,
        Operation: Fn(&T) -> Output + Send + Sync,
    {
        self.pool.install(|| items.par_iter().map(operation).collect())
    }

    /// Consume owned items in parallel, preserving their input order in the result.
    ///
    /// Use this when workers need ownership of per-item state, such as native
    /// command-list recording snapshots.
    #[must_use]
    pub fn map_owned<T, Output, Operation>(
        &self,
        items: Vec<T>,
        operation: Operation,
    ) -> Vec<Output>
    where
        T: Send,
        Output: Send,
        Operation: Fn(T) -> Output + Send + Sync,
    {
        self.pool.install(|| items.into_par_iter().map(operation).collect())
    }

    /// Partition a mutable slice into non-overlapping chunks and process them.
    ///
    /// The chunk index is stable, but execution order is unspecified. A
    /// non-zero chunk size is required so invalid scheduling parameters cannot
    /// panic inside Rayon.
    pub fn for_each_chunk_mut<T, Operation>(
        &self,
        items: &mut [T],
        chunk_size: NonZeroUsize,
        operation: Operation,
    ) where
        T: Send,
        Operation: Fn(usize, &mut [T]) + Send + Sync,
    {
        let () = self.pool.install(|| {
            items
                .par_chunks_mut(chunk_size.get())
                .enumerate()
                .for_each(|(index, chunk)| operation(index, chunk));
        });
    }

    /// Schedule two independent operations and return both results.
    ///
    /// Rayon may execute either operation on the calling thread or a worker;
    /// callers must not depend on a particular thread or execution order.
    #[must_use]
    pub fn join<Left, Right, LeftOperation, RightOperation>(
        &self,
        left: LeftOperation,
        right: RightOperation,
    ) -> (Left, Right)
    where
        Left: Send,
        Right: Send,
        LeftOperation: FnOnce() -> Left + Send,
        RightOperation: FnOnce() -> Right + Send,
    {
        self.pool.install(|| rayon::join(left, right))
    }
}

#[cfg(test)]
mod tests {
    use super::{JobSystem, JobSystemConfig};
    use std::num::NonZeroUsize;

    #[test]
    fn owned_pool_runs_jobs_and_preserves_map_order() {
        let jobs = JobSystem::new(JobSystemConfig::new(NonZeroUsize::new(2).unwrap())).unwrap();
        assert_eq!(jobs.worker_threads().get(), 2);
        assert_eq!(jobs.map(&[1, 2, 3, 4], |value| value * value), [1, 4, 9, 16]);
        assert_eq!(jobs.map_owned(vec![4, 3, 2, 1], |value| value + 1), [5, 4, 3, 2]);
    }
}
