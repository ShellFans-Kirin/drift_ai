//! Provider factory — builds a `Box<dyn CompactionProvider>` from a
//! configured provider name + the `[handoff]` / `[compaction]` config tree.
//!
//! Resolution order:
//! 1. Native built-in name (`anthropic` / `openai` / `gemini` / `ollama` / `mock`).
//! 2. User-defined entry under `providers.<name>` with `type = "openai_compatible"`.
//! 3. Otherwise → error.
//!
//! API keys are read from the env var named in the config (`api_key_env`) so
//! secrets never live in the TOML file.

use crate::compaction::openai_compat::{CustomPricing, OpenAICompatibleProvider};
use crate::compaction::{
    gemini::GeminiProvider, ollama::OllamaProvider, openai::OpenAIProvider, AnthropicProvider,
    CompactionError, CompactionProvider, LlmCompleter, MockProvider,
};
use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Per-provider config block as stored under `[handoff.providers.<name>]` or
/// `[compaction.providers.<name>]`. All fields optional so any subset is
/// accepted; missing required fields error at build time, not parse time.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ProviderConfig {
    /// `"openai_compatible"` for the generic provider; absent for native ones.
    #[serde(default)]
    pub r#type: Option<String>,
    /// Override the default model for this provider.
    #[serde(default)]
    pub model: Option<String>,
    /// Base URL for the API (used by Ollama, OpenAI-compatible).
    #[serde(default)]
    pub base_url: Option<String>,
    /// Name of the env var holding the API key. Empty / unset = keyless (e.g.
    /// LM Studio, vLLM).
    #[serde(default)]
    pub api_key_env: Option<String>,
    /// User-supplied per-1M-token input cost (USD) for OpenAI-compatible
    /// providers. Native providers ignore this and use their built-in tables.
    #[serde(default)]
    pub cost_per_1m_input_usd: Option<f64>,
    /// User-supplied per-1M-token output cost (USD).
    #[serde(default)]
    pub cost_per_1m_output_usd: Option<f64>,
}

/// Top-level routing config — `[handoff]` and `[compaction]` both have this
/// shape: a default `provider` name + a map of named provider configs.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RoutingConfig {
    /// Selected provider name. If unset, defaults to `"anthropic"`.
    #[serde(default)]
    pub provider: Option<String>,
    /// Optional per-provider override of the model (top-level shorthand).
    #[serde(default)]
    pub model: Option<String>,
    /// Named user-defined providers.
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
}

/// Build a provider from a [`RoutingConfig`]. Falls back to [`MockProvider`]
/// when the named provider can't be constructed (e.g. native provider's env
/// var is unset). Returns the box plus a flag indicating whether the fallback
/// was taken — useful for tagging output `[MOCK]` in the CLI.
pub fn make_provider(
    cfg: &RoutingConfig,
) -> Result<(Box<dyn CompactionProvider + Send + Sync>, bool), CompactionError> {
    let name = cfg
        .provider
        .as_deref()
        .unwrap_or("anthropic")
        .to_lowercase();

    // First: try native built-ins.
    if let Some(p) = build_native(&name, cfg)? {
        let mock = p.name() == "mock";
        return Ok((p, mock));
    }

    // Second: user-defined provider in `providers.<name>`.
    if let Some(pc) = cfg.providers.get(&name) {
        let kind = pc.r#type.as_deref().unwrap_or("");
        match kind {
            "openai_compatible" => {
                let provider = build_openai_compatible(&name, pc)?;
                return Ok((Box::new(provider), false));
            }
            "" => {
                return Err(CompactionError::Other(anyhow!(
                    "provider `{}` has no `type` field; set `type = \"openai_compatible\"` for a generic OpenAI-protocol provider",
                    name
                )));
            }
            other => {
                return Err(CompactionError::Other(anyhow!(
                    "provider `{}` has unknown type `{}`; valid: openai_compatible",
                    name,
                    other
                )));
            }
        }
    }

    // Third: nothing matched. Fall back to Mock so capture/handoff don't fail
    // catastrophically; the caller surfaces a `[MOCK]` tag in the output.
    Ok((Box::new(MockProvider), true))
}

/// Build a native provider if `name` is one we recognise. `Ok(None)` means
/// "name isn't a native built-in"; `Ok(Some(_))` is a built provider; `Err`
/// is a real failure (e.g. env var unset for `anthropic`).
fn build_native(
    name: &str,
    cfg: &RoutingConfig,
) -> Result<Option<Box<dyn CompactionProvider + Send + Sync>>, CompactionError> {
    let model_override = cfg
        .providers
        .get(name)
        .and_then(|pc| pc.model.clone())
        .or_else(|| cfg.model.clone());

    Ok(Some(match name {
        "mock" => Box::new(MockProvider),
        "anthropic" => match AnthropicProvider::try_new(model_override) {
            Some(p) => Box::new(p),
            None => Box::new(MockProvider),
        },
        "openai" => match OpenAIProvider::try_new(model_override) {
            Some(p) => Box::new(p),
            None => Box::new(MockProvider),
        },
        "gemini" => match GeminiProvider::try_new(model_override) {
            Some(p) => Box::new(p),
            None => Box::new(MockProvider),
        },
        "ollama" => {
            // Ollama has no API key — always constructable.
            let base = cfg
                .providers
                .get("ollama")
                .and_then(|pc| pc.base_url.clone())
                .unwrap_or_else(|| "http://localhost:11434".into());
            let model = model_override.unwrap_or_else(|| "llama3.3:70b".into());
            Box::new(OllamaProvider::new(base, model))
        }
        _ => return Ok(None),
    }))
}

/// Build an [`LlmCompleter`] for the second-pass call (used by `drift handoff`).
/// Returns `(box, mock_fallback_taken)` mirroring [`make_provider`]. When
/// `mock_fallback_taken` is true the caller should refuse to run the second
/// pass; handoff degrades to its deterministic fallback in that case.
pub fn make_completer(
    cfg: &RoutingConfig,
) -> Result<(Option<Box<dyn LlmCompleter>>, bool), CompactionError> {
    let name = cfg
        .provider
        .as_deref()
        .unwrap_or("anthropic")
        .to_lowercase();
    let model_override = cfg
        .providers
        .get(&name)
        .and_then(|pc| pc.model.clone())
        .or_else(|| cfg.model.clone());

    Ok(match name.as_str() {
        "mock" => (None, true),
        "anthropic" => match AnthropicProvider::try_new(model_override) {
            Some(p) => (Some(Box::new(p)), false),
            None => (None, true),
        },
        "openai" => match OpenAIProvider::try_new(model_override) {
            Some(p) => (Some(Box::new(p)), false),
            None => (None, true),
        },
        "gemini" => match GeminiProvider::try_new(model_override) {
            Some(p) => (Some(Box::new(p)), false),
            None => (None, true),
        },
        "ollama" => {
            let base = cfg
                .providers
                .get("ollama")
                .and_then(|pc| pc.base_url.clone())
                .unwrap_or_else(|| "http://localhost:11434".into());
            let model = model_override.unwrap_or_else(|| "llama3.3:70b".into());
            (Some(Box::new(OllamaProvider::new(base, model))), false)
        }
        other => match cfg.providers.get(other) {
            Some(pc) if pc.r#type.as_deref() == Some("openai_compatible") => {
                (Some(Box::new(build_openai_compatible(other, pc)?)), false)
            }
            _ => (None, true),
        },
    })
}

fn build_openai_compatible(
    name: &str,
    pc: &ProviderConfig,
) -> Result<OpenAICompatibleProvider, CompactionError> {
    let base_url = pc
        .base_url
        .clone()
        .ok_or_else(|| anyhow!("provider `{}` missing `base_url`", name))
        .map_err(CompactionError::Other)?;

    let model = pc
        .model
        .clone()
        .ok_or_else(|| anyhow!("provider `{}` missing `model`", name))
        .map_err(CompactionError::Other)?;

    let api_key = match pc.api_key_env.as_deref() {
        None | Some("") => None,
        Some(env_name) => std::env::var(env_name)
            .with_context(|| format!("provider `{}` requires env var `{}`", name, env_name))
            .map_err(|e| CompactionError::Other(anyhow!(e)))
            .map(Some)?,
    };

    let pricing = CustomPricing {
        input_per_mtok: pc.cost_per_1m_input_usd,
        output_per_mtok: pc.cost_per_1m_output_usd,
    };

    Ok(OpenAICompatibleProvider::new(
        name.to_string(),
        base_url,
        api_key,
        model,
        pricing,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_native_falls_back_to_mock() {
        let cfg = RoutingConfig {
            provider: Some("xai".into()),
            ..Default::default()
        };
        let (_p, mock) = make_provider(&cfg).unwrap();
        assert!(mock, "should report mock-fallback for unknown name");
    }

    #[test]
    fn missing_anthropic_key_falls_back_to_mock() {
        let prior = std::env::var("ANTHROPIC_API_KEY").ok();
        std::env::remove_var("ANTHROPIC_API_KEY");
        let cfg = RoutingConfig {
            provider: Some("anthropic".into()),
            ..Default::default()
        };
        let (p, mock) = make_provider(&cfg).unwrap();
        assert!(mock);
        assert_eq!(p.name(), "mock");
        if let Some(v) = prior {
            std::env::set_var("ANTHROPIC_API_KEY", v);
        }
    }

    #[test]
    fn ollama_always_constructable() {
        let cfg = RoutingConfig {
            provider: Some("ollama".into()),
            ..Default::default()
        };
        let (p, mock) = make_provider(&cfg).unwrap();
        assert!(!mock);
        assert_eq!(p.name(), "ollama");
    }

    #[test]
    fn openai_compatible_requires_base_url() {
        let mut providers = HashMap::new();
        providers.insert(
            "deepseek".into(),
            ProviderConfig {
                r#type: Some("openai_compatible".into()),
                model: Some("deepseek-chat".into()),
                ..Default::default()
            },
        );
        let cfg = RoutingConfig {
            provider: Some("deepseek".into()),
            providers,
            ..Default::default()
        };
        match make_provider(&cfg) {
            Err(e) => assert!(format!("{}", e).contains("base_url")),
            Ok(_) => panic!("expected error"),
        }
    }

    #[test]
    fn openai_compatible_unknown_type() {
        let mut providers = HashMap::new();
        providers.insert(
            "what".into(),
            ProviderConfig {
                r#type: Some("klingon".into()),
                ..Default::default()
            },
        );
        let cfg = RoutingConfig {
            provider: Some("what".into()),
            providers,
            ..Default::default()
        };
        match make_provider(&cfg) {
            Err(e) => assert!(format!("{}", e).contains("klingon")),
            Ok(_) => panic!("expected error"),
        }
    }

    #[test]
    fn openai_compatible_keyless_local_ok() {
        let mut providers = HashMap::new();
        providers.insert(
            "lmstudio".into(),
            ProviderConfig {
                r#type: Some("openai_compatible".into()),
                base_url: Some("http://localhost:1234/v1".into()),
                model: Some("local-model".into()),
                api_key_env: None,
                ..Default::default()
            },
        );
        let cfg = RoutingConfig {
            provider: Some("lmstudio".into()),
            providers,
            ..Default::default()
        };
        let (p, mock) = make_provider(&cfg).unwrap();
        assert!(!mock);
        assert_eq!(p.name(), "openai-compatible");
    }
}
