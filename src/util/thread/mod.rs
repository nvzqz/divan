//! Threading utilities.

pub(crate) mod local;

mod pool;

pub(crate) use pool::ThreadPool;
