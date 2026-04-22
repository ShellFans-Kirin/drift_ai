//! drift-core — data model, attribution engine, compaction trait, and storage
//! for Drift AI.
//!
//! Public surface is intentionally narrow: the session + code-event model,
//! the SQLite store, the `SessionConnector` trait (re-exported into
//! `drift-connectors`), and the `CompactionProvider` trait. CLI and MCP
//! crates consume this crate; connector implementations live in
//! `drift-connectors`.

pub mod attribution;
pub mod compaction;
pub mod config;
pub mod diff;
pub mod git;
pub mod model;
pub mod shell_lexer;
pub mod store;

pub use model::{
    AgentSlug, CodeEvent, NormalizedSession, Operation, Role, Turn, ToolCall, ToolResult,
};
pub use store::EventStore;
