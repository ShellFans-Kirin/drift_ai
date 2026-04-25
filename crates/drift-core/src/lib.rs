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
pub mod handoff;
pub mod model;
pub mod shell_lexer;
pub mod store;

pub use compaction::{
    compute_cost_usd, pricing_for, AnthropicProvider, CompactedSummary, CompactionError,
    CompactionProvider, CompactionRes, CompactionResult, CompactionUsage, LlmCompletion,
    MockProvider, ModelPricing,
};
pub use handoff::{
    build_handoff, render_brief, Decision, ExcerptKind, FileSnippet, HandoffBrief, HandoffOptions,
    HandoffScope, ProgressItem, ProgressStatus, RejectedApproach, SessionSlim, TargetAgent,
};
pub use model::{
    AgentSlug, CodeEvent, NormalizedSession, Operation, Role, ToolCall, ToolResult, Turn,
};
pub use store::{CostFilter, CostGroupBy, CostGroupRow, CostTotals, EventStore};
