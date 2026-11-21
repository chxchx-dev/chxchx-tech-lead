//! Jobs that outlive the window that started them (ADR 0017, H1): a persistent store and a
//! runner. The wire types come from `chxchx-protocol`.

mod lock;
pub mod runner;
pub mod store;

#[cfg(test)]
mod tests;

pub use runner::{Context, Executor, JobRunner};
pub use store::{JobStore, default_dir, read, read_events};
