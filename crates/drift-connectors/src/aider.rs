//! Aider connector — stub.
//!
//! Feature-gated behind `aider`. Shipped as the worked example for
//! "adding a new connector" in CONTRIBUTING.md.
//! TODO: implement once the aider jsonl layout is confirmed.

use super::{SessionConnector, SessionRef};
use anyhow::Result;
use drift_core::attribution::CodeEventDraft;
use drift_core::NormalizedSession;

pub struct AiderConnector;

impl SessionConnector for AiderConnector {
    fn agent_slug(&self) -> &'static str {
        "aider"
    }
    fn discover(&self) -> Result<Vec<SessionRef>> {
        Ok(vec![])
    }
    fn parse(&self, _r: &SessionRef) -> Result<NormalizedSession> {
        anyhow::bail!("aider connector not yet implemented; see CONTRIBUTING.md")
    }
    fn extract_code_events(&self, _ns: &NormalizedSession) -> Result<Vec<CodeEventDraft>> {
        Ok(vec![])
    }
}
