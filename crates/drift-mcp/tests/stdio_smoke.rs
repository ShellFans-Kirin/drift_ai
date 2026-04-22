//! Stdio round-trip: send initialize + tools/list over a pipe and assert
//! the server replies with a well-formed JSON-RPC envelope.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn binary() -> PathBuf {
    // Integration tests run with CARGO_BIN_EXE_<name> only for binaries
    // owned by the SAME crate. drift-mcp is a library crate, so we shell
    // out to the workspace binary at the canonical target location.
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.pop(); // repo root
    p.push("target");
    p.push(if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    });
    p.push("drift");
    p
}

#[test]
#[ignore = "requires workspace build; run with --ignored after cargo build"]
fn stdio_initialize_then_tools_list() {
    let bin = binary();
    if !bin.exists() {
        eprintln!("skipping: drift binary not present at {}", bin.display());
        return;
    }
    let tempdir = tempfile::tempdir().unwrap();
    let mut child = Command::new(&bin)
        .arg("--repo")
        .arg(tempdir.path())
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn drift mcp");
    {
        let stdin = child.stdin.as_mut().unwrap();
        writeln!(stdin, r#"{{"jsonrpc":"2.0","id":1,"method":"initialize"}}"#).unwrap();
        writeln!(stdin, r#"{{"jsonrpc":"2.0","id":2,"method":"tools/list"}}"#).unwrap();
    }
    drop(child.stdin.take());
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    let mut lines = Vec::new();
    for _ in 0..2 {
        let mut s = String::new();
        reader.read_line(&mut s).unwrap();
        lines.push(s);
    }
    child.wait().unwrap();

    let init: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
    assert_eq!(init["jsonrpc"], "2.0");
    assert_eq!(init["result"]["serverInfo"]["name"], "drift");

    let tools: serde_json::Value = serde_json::from_str(&lines[1]).unwrap();
    let names: Vec<&str> = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    for want in [
        "drift_blame",
        "drift_trace",
        "drift_rejected",
        "drift_log",
        "drift_show_event",
    ] {
        assert!(names.contains(&want), "missing tool {}", want);
    }
}
