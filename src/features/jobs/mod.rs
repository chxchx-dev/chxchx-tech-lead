//! Jobs that outlive the window that started them (ADR 0017, H1): a persistent store, a runner
//! and the executor of the product's own jobs. The wire types come from `chxchx-protocol`.

pub mod agent_turn;
mod lock;
pub mod product;
pub mod runner;
pub mod store;

#[cfg(test)]
mod agent_turn_tests;
#[cfg(test)]
mod tests;

pub use product::ProductExecutor;
pub use runner::{Context, Executor, JobRunner, MAX_QUEUED};
pub use store::{JobStore, default_dir, read, read_events};
