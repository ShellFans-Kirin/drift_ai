//! Config file schema for `drift`.
//!
//! Global: `~/.config/drift/config.toml`
//! Project: `<repo>/.prompts/config.toml` (overrides)

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftConfig {
    #[serde(default)]
    pub attribution: AttributionConfig,
    #[serde(default)]
    pub connectors: ConnectorsConfig,
    #[serde(default)]
    pub compaction: CompactionConfig,
    #[serde(default)]
    pub handoff: HandoffConfig,
    #[serde(default)]
    pub sync: SyncConfig,
}

impl Default for DriftConfig {
    fn default() -> Self {
        Self {
            attribution: AttributionConfig::default(),
            connectors: ConnectorsConfig::default(),
            compaction: CompactionConfig::default(),
            handoff: HandoffConfig::default(),
            sync: SyncConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributionConfig {
    #[serde(default = "default_true")]
    pub db_in_git: bool,
}
impl Default for AttributionConfig {
    fn default() -> Self {
        Self { db_in_git: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorsConfig {
    #[serde(default = "default_true")]
    pub claude_code: bool,
    #[serde(default = "default_true")]
    pub codex: bool,
    #[serde(default)]
    pub aider: bool,
}
impl Default for ConnectorsConfig {
    fn default() -> Self {
        Self {
            claude_code: true,
            codex: true,
            aider: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionConfig {
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default)]
    pub provider: Option<String>, // "anthropic" | "mock"
}
impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            model: "claude-opus-4-7".to_string(),
            provider: None,
        }
    }
}

/// Settings for `drift handoff` (v0.2.0+). Currently only the Anthropic
/// model is configurable. Default `claude-opus-4-7` because the brief
/// is a user-facing artifact the next agent reads verbatim — narrative
/// quality matters here far more than for per-session compaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffConfig {
    #[serde(default = "default_handoff_model")]
    pub model: String,
}

impl Default for HandoffConfig {
    fn default() -> Self {
        Self {
            model: default_handoff_model(),
        }
    }
}

fn default_handoff_model() -> String {
    "claude-opus-4-7".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncConfig {
    #[serde(default)]
    pub notes_remote: Option<String>,
}

fn default_true() -> bool {
    true
}
fn default_model() -> String {
    "claude-opus-4-7".to_string()
}

pub fn global_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("drift").join("config.toml"))
}

pub fn project_config_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".prompts").join("config.toml")
}

pub fn load(repo_root: &Path) -> Result<DriftConfig> {
    let mut cfg = DriftConfig::default();
    if let Some(g) = global_config_path() {
        if g.exists() {
            let text = std::fs::read_to_string(&g)
                .with_context(|| format!("read global config {}", g.display()))?;
            cfg = toml::from_str(&text)
                .with_context(|| format!("parse global config {}", g.display()))?;
        }
    }
    let p = project_config_path(repo_root);
    if p.exists() {
        let text = std::fs::read_to_string(&p)
            .with_context(|| format!("read project config {}", p.display()))?;
        let proj: DriftConfig = toml::from_str(&text)
            .with_context(|| format!("parse project config {}", p.display()))?;
        // project wins: overlay non-defaults (shallow — sufficient for v0.1.0)
        cfg.attribution = proj.attribution;
        cfg.connectors = proj.connectors;
        cfg.compaction = proj.compaction;
        cfg.sync = proj.sync;
    }
    Ok(cfg)
}

pub fn write_project_default(repo_root: &Path) -> Result<PathBuf> {
    let p = project_config_path(repo_root);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let defaults = DriftConfig::default();
    let toml_s = toml::to_string_pretty(&defaults)?;
    std::fs::write(&p, toml_s)?;
    Ok(p)
}
