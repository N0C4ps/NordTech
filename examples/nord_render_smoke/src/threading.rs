//! Stable application facade for engine worker pools.

pub use nordtech::threading::{
    ThreadPool as ApplicationThreadPool, ThreadPoolBuildError as ApplicationThreadPoolBuildError,
    ThreadPoolConfig as ApplicationThreadPoolConfig,
};
