//! Phase 1C real-API smoke tests.
//!
//! Each test is `#[ignore]` so `cargo test` (no flags) skips them. Run with:
//!   cargo test --test v030_real_smoke -- --ignored --nocapture
//! Each test self-skips when its API key env var is unset, so partial coverage
//! works (DeepSeek-only run is fine).
//!
//! Each test prints a single MARKDOWN row to stdout that the smoke harness
//! collects into `docs/V030-V040-SMOKE.md` for the launch report.

use chrono::{TimeZone, Utc};
use drift_core::compaction::factory::{ProviderConfig, RoutingConfig};
use drift_core::compaction::{
    gemini::GeminiProvider,
    ollama::OllamaProvider,
    openai::OpenAIProvider,
    openai_compat::{CustomPricing, OpenAICompatibleProvider},
    AnthropicProvider, CompactionProvider,
};
use drift_core::model::{AgentSlug, NormalizedSession, Role, ToolCall, Turn};
use serde_json::json;
use std::collections::HashMap;
use std::time::Instant;

fn fixture_session() -> NormalizedSession {
    NormalizedSession {
        session_id: "v030-smoke-fixture".into(),
        agent_slug: AgentSlug::ClaudeCode,
        model: Some("(provider-under-test)".into()),
        working_dir: None,
        git_head_at_start: None,
        started_at: Utc.with_ymd_and_hms(2026, 4, 25, 10, 0, 0).unwrap(),
        ended_at: Utc.with_ymd_and_hms(2026, 4, 25, 10, 5, 0).unwrap(),
        turns: vec![
            Turn {
                turn_id: "t1".into(),
                role: Role::User,
                content_text: "Add a rate limiter to the login route".into(),
                tool_calls: vec![],
                tool_results: vec![],
                timestamp: Utc.with_ymd_and_hms(2026, 4, 25, 10, 1, 0).unwrap(),
            },
            Turn {
                turn_id: "t2".into(),
                role: Role::Assistant,
                content_text: "I'll add a sliding window rate limiter on src/auth/login.ts. \
                     Rejected: per-IP token bucket (too memory heavy for our scale). \
                     Decided: 5 attempts per minute, then 423 Locked."
                    .into(),
                tool_calls: vec![ToolCall {
                    id: "tc1".into(),
                    name: "Edit".into(),
                    input: json!({ "file_path": "src/auth/login.ts" }),
                }],
                tool_results: vec![],
                timestamp: Utc.with_ymd_and_hms(2026, 4, 25, 10, 2, 0).unwrap(),
            },
            Turn {
                turn_id: "t3".into(),
                role: Role::User,
                content_text: "Looks good. Tests next.".into(),
                tool_calls: vec![],
                tool_results: vec![],
                timestamp: Utc.with_ymd_and_hms(2026, 4, 25, 10, 3, 0).unwrap(),
            },
            Turn {
                turn_id: "t4".into(),
                role: Role::Assistant,
                content_text: "Adding tests in src/auth/login.test.ts: \
                               attempts <5 returns 200, 6th returns 423, \
                               minute window resets the counter."
                    .into(),
                tool_calls: vec![ToolCall {
                    id: "tc2".into(),
                    name: "Write".into(),
                    input: json!({ "file_path": "src/auth/login.test.ts" }),
                }],
                tool_results: vec![],
                timestamp: Utc.with_ymd_and_hms(2026, 4, 25, 10, 4, 0).unwrap(),
            },
        ],
        thinking_blocks: 0,
    }
}

fn report_row(provider: &str, model: &str, t: &Instant, res: drift_core::CompactionResult) {
    let elapsed = t.elapsed().as_millis();
    let usage = res.usage.as_ref();
    let (it, ot, cost) = usage
        .map(|u| (u.input_tokens, u.output_tokens, u.cost_usd))
        .unwrap_or((0, 0, 0.0));
    let preview: String = res.summary.summary.chars().take(80).collect();
    println!(
        "| {} | {} | {} ms | {} | {} | ${:.6} | {} |",
        provider,
        model,
        elapsed,
        it,
        ot,
        cost,
        preview.replace('|', "/").replace('\n', " ")
    );
}

#[test]
#[ignore]
fn smoke_anthropic() {
    if std::env::var("ANTHROPIC_API_KEY")
        .ok()
        .filter(|s| !s.is_empty())
        .is_none()
    {
        println!("| anthropic | (skipped) | — | — | — | — | ANTHROPIC_API_KEY unset |");
        return;
    }
    std::env::set_var("DRIFT_COMPACT_QUIET", "1");
    let provider =
        AnthropicProvider::try_new(Some("claude-haiku-4-5".into())).expect("provider build");
    let session = fixture_session();
    let t = Instant::now();
    let res = provider.compact(&session).expect("anthropic smoke");
    report_row("anthropic", "claude-haiku-4-5", &t, res);
}

#[test]
#[ignore]
fn smoke_openai() {
    if std::env::var("OPENAI_API_KEY")
        .ok()
        .filter(|s| !s.is_empty())
        .is_none()
    {
        println!("| openai | (skipped) | — | — | — | — | OPENAI_API_KEY unset |");
        return;
    }
    let provider = OpenAIProvider::try_new(Some("gpt-4o-mini".into())).expect("provider build");
    let session = fixture_session();
    let t = Instant::now();
    let res = provider.compact(&session).expect("openai smoke");
    report_row("openai", "gpt-4o-mini", &t, res);
}

#[test]
#[ignore]
fn smoke_gemini() {
    if std::env::var("GEMINI_API_KEY")
        .ok()
        .filter(|s| !s.is_empty())
        .is_none()
    {
        println!("| gemini | (skipped) | — | — | — | — | GEMINI_API_KEY unset |");
        return;
    }
    // gemini-2.5-flash with thinkingBudget=0 (set by GeminiProvider) so the
    // output budget isn't drained on hidden thoughts. Pro on the free tier
    // is heavily rate-limited; Flash is a better default smoke target.
    let provider =
        GeminiProvider::try_new(Some("gemini-2.5-flash".into())).expect("provider build");
    let session = fixture_session();
    let t = Instant::now();
    let res = provider.compact(&session).expect("gemini smoke");
    report_row("gemini", "gemini-2.5-flash", &t, res);
}

#[test]
#[ignore]
fn smoke_deepseek_via_openai_compatible() {
    let key = match std::env::var("DEEPSEEK_API_KEY")
        .ok()
        .filter(|s| !s.is_empty())
    {
        Some(k) => k,
        None => {
            println!("| deepseek (compat) | (skipped) | — | — | — | — | DEEPSEEK_API_KEY unset |");
            return;
        }
    };
    let provider = OpenAICompatibleProvider::new(
        "deepseek",
        "https://api.deepseek.com",
        Some(key),
        "deepseek-chat",
        CustomPricing {
            input_per_mtok: Some(0.27),
            output_per_mtok: Some(1.10),
        },
    );
    let session = fixture_session();
    let t = Instant::now();
    let res = provider.compact(&session).expect("deepseek smoke");
    report_row("deepseek (compat)", "deepseek-chat", &t, res);
}

#[test]
#[ignore]
fn smoke_ollama() {
    // Probe whether ollama is running before we try.
    let probe = std::process::Command::new("curl")
        .args(["-sf", "--max-time", "1", "http://localhost:11434/api/tags"])
        .output();
    if !probe.map(|o| o.status.success()).unwrap_or(false) {
        println!("| ollama | (skipped) | — | — | — | — | daemon not running |");
        return;
    }
    let provider = OllamaProvider::default();
    let session = fixture_session();
    let t = Instant::now();
    let res = provider.compact(&session).expect("ollama smoke");
    report_row("ollama", &provider.model, &t, res);
}

#[test]
#[ignore]
fn smoke_factory_resolves_named_provider() {
    // Independent of network — confirms RoutingConfig + named provider entries
    // resolve correctly when DEEPSEEK_API_KEY is present.
    if std::env::var("DEEPSEEK_API_KEY")
        .ok()
        .filter(|s| !s.is_empty())
        .is_none()
    {
        println!("(skipped: factory smoke requires DEEPSEEK_API_KEY)");
        return;
    }
    let mut providers = HashMap::new();
    providers.insert(
        "deepseek".into(),
        ProviderConfig {
            r#type: Some("openai_compatible".into()),
            base_url: Some("https://api.deepseek.com".into()),
            model: Some("deepseek-chat".into()),
            api_key_env: Some("DEEPSEEK_API_KEY".into()),
            cost_per_1m_input_usd: Some(0.27),
            cost_per_1m_output_usd: Some(1.10),
        },
    );
    let cfg = RoutingConfig {
        provider: Some("deepseek".into()),
        providers,
        ..Default::default()
    };
    let (p, mock) = drift_core::compaction::factory::make_provider(&cfg).unwrap();
    assert!(!mock);
    assert_eq!(p.name(), "openai-compatible");
}
