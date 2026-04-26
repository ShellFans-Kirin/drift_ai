//! Config file schema for `drift`.
//!
//! Global: `~/.config/drift/config.toml`
//! Project: `<repo>/.prompts/config.toml` (overrides)

use crate::compaction::factory::{ProviderConfig, RoutingConfig};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    pub cursor: bool,
    #[serde(default)]
    pub aider: bool,
}
impl Default for ConnectorsConfig {
    fn default() -> Self {
        Self {
            claude_code: true,
            codex: true,
            cursor: false,
            aider: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionConfig {
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default)]
    pub provider: Option<String>, // "anthropic" | "openai" | "gemini" | "ollama" | "mock" | <named>
    /// v0.3+: named provider entries (`[compaction.providers.<name>]`).
    /// Empty in v0.2-style configs; ignored unless `provider` references one.
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
}
impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            model: "claude-opus-4-7".to_string(),
            provider: None,
            providers: HashMap::new(),
        }
    }
}

impl CompactionConfig {
    pub fn to_routing(&self) -> RoutingConfig {
        RoutingConfig {
            provider: self.provider.clone(),
            model: Some(self.model.clone()),
            providers: self.providers.clone(),
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
    /// v0.3+: provider selector. Defaults to the same selection as
    /// `[compaction]` if omitted, which in turn defaults to `"anthropic"`.
    #[serde(default)]
    pub provider: Option<String>,
    /// v0.3+: named provider entries.
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
}

impl Default for HandoffConfig {
    fn default() -> Self {
        Self {
            model: default_handoff_model(),
            provider: None,
            providers: HashMap::new(),
        }
    }
}

impl HandoffConfig {
    pub fn to_routing(&self) -> RoutingConfig {
        RoutingConfig {
            provider: self.provider.clone(),
            model: Some(self.model.clone()),
            providers: self.providers.clone(),
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
    std::fs::write(&p, DEFAULT_CONFIG_TEMPLATE)?;
    Ok(p)
}

/// v0.3+ default config: minimal active settings (anthropic-only, identical
/// to v0.2 behaviour) plus commented-out templates for every other supported
/// provider, so a user can switch by uncommenting + setting an env var.
pub const DEFAULT_CONFIG_TEMPLATE: &str = r##"# drift_ai project config
# Global merge: ~/.config/drift/config.toml is loaded first, this file overrides.

[attribution]
db_in_git = true             # default — share blame with the team via the repo

[connectors]
claude_code = true
codex       = true
cursor      = false          # v0.4 (best-effort, undocumented Cursor schema)
aider       = false          # v0.4 full impl

# ---------------------------------------------------------------------------
# [compaction] — drift capture's per-session LLM call
# ---------------------------------------------------------------------------

[compaction]
provider = "anthropic"
model    = "claude-haiku-4-5"   # cheap default; switch to opus for quality

# Native providers (uncomment one + set the named env var to switch):

# [compaction.providers.openai]
# model = "gpt-5"
# api_key_env = "OPENAI_API_KEY"

# [compaction.providers.gemini]
# model = "gemini-2.5-pro"
# api_key_env = "GEMINI_API_KEY"

# [compaction.providers.ollama]
# base_url = "http://localhost:11434"
# model = "llama3.3:70b"

# OpenAI-protocol generic providers:

# [compaction.providers.deepseek]
# type = "openai_compatible"
# base_url = "https://api.deepseek.com"
# model = "deepseek-chat"
# api_key_env = "DEEPSEEK_API_KEY"
# cost_per_1m_input_usd = 0.27
# cost_per_1m_output_usd = 1.10

# [compaction.providers.groq]
# type = "openai_compatible"
# base_url = "https://api.groq.com/openai/v1"
# model = "llama-3.3-70b-versatile"
# api_key_env = "GROQ_API_KEY"

# [compaction.providers.mistral]
# type = "openai_compatible"
# base_url = "https://api.mistral.ai/v1"
# model = "mistral-large-latest"
# api_key_env = "MISTRAL_API_KEY"

# [compaction.providers.together]
# type = "openai_compatible"
# base_url = "https://api.together.xyz/v1"
# model = "meta-llama/Llama-3.3-70B-Instruct-Turbo"
# api_key_env = "TOGETHER_API_KEY"

# [compaction.providers.lmstudio]
# type = "openai_compatible"
# base_url = "http://localhost:1234/v1"
# model = "local-model"
# # api_key_env not set — LM Studio is keyless

# [compaction.providers.vllm]
# type = "openai_compatible"
# base_url = "http://localhost:8000/v1"
# model = "your-model"

# ---------------------------------------------------------------------------
# [handoff] — drift handoff's brief-generation LLM call
# ---------------------------------------------------------------------------

[handoff]
provider = "anthropic"
model    = "claude-opus-4-7"    # narrative quality matters here; ~30x cheaper Haiku also fine

# Same `providers.<name>` shape as [compaction.providers.<name>]; uncomment
# and `provider = "<name>"` above to route handoff briefs through a different
# LLM (e.g. DeepSeek for ~30x cost reduction at similar narrative quality).
# Examples are intentionally omitted from [handoff.*] to keep this file
# readable — copy from [compaction.providers.*] above.

[sync]
# notes_remote = "origin"
"##;
