//! Git for the project: reading its state, the history graph, guidance and changes.
pub mod graph;
pub mod guide;
pub mod operations;
pub mod parse;
pub mod plan;
pub mod problem;
pub mod repository;
pub mod suggest;
pub mod types;

#[cfg(test)]
mod tests;
#[cfg(test)]
#[path = "tests_plan.rs"]
mod tests_plan;

pub use operations::PullMode;
pub use problem::{GitFailure, Problem};
pub use repository::Repository;
pub use types::{GitBranch, GitCommit, GitEntry, GitFileChange, GitStash, GitStatus};
