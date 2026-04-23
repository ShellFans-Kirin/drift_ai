//! `drift cost` — aggregate compaction_calls and print token/USD totals.

use super::open_store;
use anyhow::Result;
use drift_core::{CostFilter, CostGroupBy};
use std::path::Path;

pub fn run(
    repo: &Path,
    since: Option<&str>,
    until: Option<&str>,
    model: Option<&str>,
    by: Option<&str>,
) -> Result<()> {
    let store = open_store(repo)?;
    let filter = CostFilter {
        since: since.map(String::from),
        until: until.map(String::from),
        model: model.map(String::from),
    };

    let totals = store.query_cost(&filter)?;
    println!("drift cost — compaction billing");
    if let Some(s) = since {
        println!("  since : {}", s);
    }
    if let Some(u) = until {
        println!("  until : {}", u);
    }
    if let Some(m) = model {
        println!("  model : {}", m);
    }
    println!();
    println!("  total calls      : {}", totals.calls);
    println!("  input tokens     : {}", totals.input_tokens);
    println!("  output tokens    : {}", totals.output_tokens);
    println!("  cache creation   : {}", totals.cache_creation_tokens);
    println!("  cache read       : {}", totals.cache_read_tokens);
    println!("  total cost (USD) : ${:.4}", totals.total_cost_usd);

    if let Some(by_raw) = by {
        let group_by = match by_raw.to_lowercase().as_str() {
            "model" => CostGroupBy::Model,
            "session" => CostGroupBy::Session,
            "date" => CostGroupBy::Date,
            other => anyhow::bail!("--by expects one of model|session|date, got `{}`", other),
        };
        let rows = store.query_cost_grouped(&filter, group_by)?;
        println!();
        println!("── grouped by {} (descending cost)", by_raw);
        println!(
            "  {:<40}  {:>6}  {:>12}  {:>12}  {:>14}",
            "key", "calls", "input_tok", "output_tok", "cost (USD)"
        );
        for r in rows {
            let key = if r.key.len() > 40 {
                format!("{}…", &r.key[..39])
            } else {
                r.key
            };
            println!(
                "  {:<40}  {:>6}  {:>12}  {:>12}  {:>14}",
                key,
                r.calls,
                r.input_tokens,
                r.output_tokens,
                format!("${:.4}", r.total_cost_usd)
            );
        }
    }

    if totals.calls == 0 {
        println!();
        println!(
            "(no compaction_calls recorded yet — run `drift capture` with ANTHROPIC_API_KEY set)"
        );
    }

    Ok(())
}
