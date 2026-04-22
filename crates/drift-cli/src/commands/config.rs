use anyhow::Result;
use drift_core::config as cfg;
use std::path::Path;

pub fn get(repo: &Path, key: &str) -> Result<()> {
    let c = cfg::load(repo)?;
    let v = serde_json::to_value(&c)?;
    if let Some(found) = walk(&v, key) {
        println!("{}", found);
    } else {
        println!("(unset)");
    }
    Ok(())
}

pub fn set(repo: &Path, key: &str, value: &str) -> Result<()> {
    let p = cfg::project_config_path(repo);
    if !p.exists() {
        cfg::write_project_default(repo)?;
    }
    let text = std::fs::read_to_string(&p)?;
    let mut v: toml::Value = toml::from_str(&text)?;
    set_toml(&mut v, key, value)?;
    std::fs::write(&p, toml::to_string_pretty(&v)?)?;
    println!("set {} = {} (project)", key, value);
    Ok(())
}

pub fn list(repo: &Path) -> Result<()> {
    let c = cfg::load(repo)?;
    println!("{}", toml::to_string_pretty(&c)?);
    Ok(())
}

fn walk<'a>(v: &'a serde_json::Value, key: &str) -> Option<String> {
    let mut cur = v;
    for part in key.split('.') {
        cur = cur.get(part)?;
    }
    Some(match cur {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    })
}

fn set_toml(v: &mut toml::Value, key: &str, value: &str) -> Result<()> {
    let parts: Vec<&str> = key.split('.').collect();
    let mut cur = v;
    for p in &parts[..parts.len() - 1] {
        let table = cur
            .as_table_mut()
            .ok_or_else(|| anyhow::anyhow!("not a table at {}", p))?;
        cur = table
            .entry(p.to_string())
            .or_insert_with(|| toml::Value::Table(Default::default()));
    }
    let table = cur
        .as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("not a table at {}", parts[parts.len() - 2]))?;
    let last = parts.last().unwrap();
    // Try bool/int/string in that order.
    let val = if let Ok(b) = value.parse::<bool>() {
        toml::Value::Boolean(b)
    } else if let Ok(i) = value.parse::<i64>() {
        toml::Value::Integer(i)
    } else {
        toml::Value::String(value.to_string())
    };
    table.insert(last.to_string(), val);
    Ok(())
}
