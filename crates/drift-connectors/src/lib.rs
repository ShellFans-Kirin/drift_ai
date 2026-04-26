//! Session connectors. One trait, multiple implementations.
//!
//! The trait contract matches PROPOSAL §E.1:
//! - `discover()` enumerates candidate session files in the agent's default
//!   on-disk location.
//! - `parse()` reads one file into an opaque `RawSession`.
//! - `normalize()` flattens a `RawSession` into the shared
//!   [`drift_core::NormalizedSession`].
//! - `extract_code_events()` walks a normalised session and emits
//!   [`drift_core::attribution::CodeEventDraft`]s for each file operation.

use anyhow::Result;
use drift_core::attribution::CodeEventDraft;
use drift_core::NormalizedSession;
use std::path::PathBuf;

#[cfg(feature = "aider")]
pub mod aider;
pub mod claude_code;
pub mod codex;
#[cfg(feature = "cursor")]
pub mod cursor;

/// Reference to a discovered session file.
#[derive(Debug, Clone)]
pub struct SessionRef {
    pub agent_slug: &'static str,
    pub path: PathBuf,
}

pub trait SessionConnector {
    fn agent_slug(&self) -> &'static str;
    fn discover(&self) -> Result<Vec<SessionRef>>;
    fn parse(&self, r: &SessionRef) -> Result<NormalizedSession>;
    fn extract_code_events(&self, ns: &NormalizedSession) -> Result<Vec<CodeEventDraft>>;
}

/// Build the default set of connectors based on compile-time features.
pub fn default_connectors() -> Vec<Box<dyn SessionConnector>> {
    let mut v: Vec<Box<dyn SessionConnector>> = Vec::new();
    #[cfg(feature = "claude-code")]
    v.push(Box::new(
        claude_code::ClaudeCodeConnector::with_default_root(),
    ));
    #[cfg(feature = "codex")]
    v.push(Box::new(codex::CodexConnector::with_default_root()));
    #[cfg(feature = "cursor")]
    v.push(Box::new(cursor::CursorConnector::with_default_root()));
    #[cfg(feature = "aider")]
    v.push(Box::new(aider::AiderConnector));
    v
}
