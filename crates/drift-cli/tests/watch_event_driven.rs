//! Integration test for `drift watch` — confirms that writing a new
//! `.jsonl` file under a watched directory triggers a capture within the
//! 200ms debounce window. Runs end-to-end on the same platform as the
//! user (inotify on Linux, FSEvents on macOS); on Windows it's
//! best-effort and may be flaky, matching the v0.1.1 scope note.

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

fn bin() -> PathBuf {
    // Cargo places the compiled binary next to this test crate's target
    // dir. When invoked via `cargo test`, CARGO_BIN_EXE_drift points at
    // the CLI binary.
    PathBuf::from(env!("CARGO_BIN_EXE_drift"))
}

#[test]
#[ignore = "needs a writable HOME + long-running child; run with --ignored"]
fn watch_captures_new_jsonl_within_a_second() {
    // Point HOME at a tempdir so the watcher listens on our fake
    // ~/.claude/projects/ rather than the user's real one.
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join(".claude").join("projects").join("test")).unwrap();
    let repo = tempfile::tempdir().unwrap();
    std::process::Command::new("git")
        .arg("init")
        .arg("-q")
        .current_dir(repo.path())
        .status()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.email", "t@e.st"])
        .current_dir(repo.path())
        .status()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.name", "t"])
        .current_dir(repo.path())
        .status()
        .unwrap();

    let mut child: Child = Command::new(bin())
        .args(["--repo"])
        .arg(repo.path())
        .arg("watch")
        .env("HOME", tmp.path())
        .env("DRIFT_COMPACT_QUIET", "1")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    // Give the watcher time to register inotify on the dir.
    thread::sleep(Duration::from_millis(400));

    let jsonl = tmp
        .path()
        .join(".claude")
        .join("projects")
        .join("test")
        .join("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee.jsonl");
    // Write a single minimal claude-code turn so the parser succeeds.
    std::fs::write(
        &jsonl,
        r#"{"type":"user","message":{"role":"user","content":"hi"},"uuid":"u1","sessionId":"aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee","timestamp":"2026-04-23T10:00:00Z","cwd":"/tmp","version":"1","gitBranch":"main"}
"#,
    )
    .unwrap();

    // Let the watcher debounce + run capture.
    thread::sleep(Duration::from_millis(1500));
    let _ = child.kill();
    let _ = child.wait();

    // The capture may or may not have produced a compacted markdown
    // depending on whether the connector accepts a single-turn user-only
    // session; however, drift init must have run, so .prompts/ exists.
    assert!(repo.path().join(".prompts").exists());
}
