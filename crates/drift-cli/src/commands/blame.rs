use super::open_store;
use anyhow::Result;
use drift_core::CodeEvent;
use std::path::Path;

pub fn run(repo: &Path, file: &Path, line: Option<u32>, range: Option<&str>) -> Result<()> {
    let store = open_store(repo)?;
    let rel = file.strip_prefix(repo).unwrap_or(file).to_string_lossy().to_string();
    let events = store.events_for_file(&rel)?;
    if events.is_empty() {
        println!("(no events recorded for {})", rel);
        return Ok(());
    }

    let target_range = if let Some(l) = line {
        Some((l, l))
    } else if let Some(r) = range {
        let parts: Vec<&str> = r.split('-').collect();
        if parts.len() == 2 {
            let a: u32 = parts[0].parse().unwrap_or(0);
            let b: u32 = parts[1].parse().unwrap_or(a);
            Some((a, b))
        } else {
            None
        }
    } else {
        None
    };

    println!("{}", rel);
    for ev in &events {
        if let Some(rng) = target_range {
            if !touches(ev, rng) {
                continue;
            }
        }
        print_event(ev);
    }
    Ok(())
}

fn touches(ev: &CodeEvent, rng: (u32, u32)) -> bool {
    let (s, e) = rng;
    ev.line_ranges_after
        .iter()
        .any(|(a, b)| *a <= e && *b >= s)
        || ev
            .line_ranges_before
            .iter()
            .any(|(a, b)| *a <= e && *b >= s)
}

fn print_event(ev: &CodeEvent) {
    let ts = ev.timestamp.format("%Y-%m-%d %H:%M");
    let agent = ev.agent_slug.as_str();
    let session = ev
        .session_id
        .as_deref()
        .map(|s| format!("session {}", s.chars().take(8).collect::<String>()))
        .unwrap_or_else(|| "post-commit manual edit".to_string());
    let glyph = match ev.agent_slug {
        drift_core::model::AgentSlug::Human => "✋",
        _ => "💭",
    };
    println!(
        "├─ {}  {} [{}] {}",
        ts, glyph, agent, session
    );
    if !ev.diff_hunks.is_empty() {
        for line in ev.diff_hunks.lines().take(8) {
            println!("│   {}", line);
        }
    }
    if ev.rejected {
        println!("│   (rejected suggestion)");
    }
}
