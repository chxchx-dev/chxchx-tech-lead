//! Colmena (ADR 0018): the board, the decisions and, later, the search index of one project.
//!
//! Nothing here depends on the interface. The database lives in the application data folder, one
//! per project, and is only opened when the extension is on.
pub mod store;
