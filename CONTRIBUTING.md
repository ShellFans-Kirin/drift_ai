# Contributing to drift_ai

Thanks for looking. Before anything else, please skim
[`docs/VISION.md`](docs/VISION.md) and
[`docs/PHASE0-PROPOSAL.md`](docs/PHASE0-PROPOSAL.md) §D — the data
model is the spine of the project and the thing least forgiving of
drift.

## Development setup

```bash
git clone https://github.com/ShellFans-Kirin/drift_ai.git
cd drift_ai
cargo test --workspace
cargo build --workspace --release
./target/release/drift --version
```

Rust 1.85+ is required (some transitive deps need edition2024).

## Adding a new connector

Each AI coding agent gets a connector module in `crates/drift-connectors/`.
The contract is the [`SessionConnector`
trait](crates/drift-connectors/src/lib.rs):

```rust
trait SessionConnector {
    fn agent_slug(&self) -> &'static str;
    fn discover(&self) -> Result<Vec<SessionRef>>;
    fn parse(&self, r: &SessionRef) -> Result<NormalizedSession>;
    fn extract_code_events(&self, ns: &NormalizedSession) -> Result<Vec<CodeEventDraft>>;
}
```

The two worked examples are Aider
([`crates/drift-connectors/src/aider.rs`](crates/drift-connectors/src/aider.rs))
and Cursor
([`crates/drift-connectors/src/cursor.rs`](crates/drift-connectors/src/cursor.rs)).
Aider parses plain markdown; Cursor reads SQLite. Pick the closer match for
your target agent.

The general steps:

1. **Find the on-disk session layout.** Capture a real sample first.
   Examples in this repo:
   - Claude Code: `~/.claude/projects/<workspace>/<session>.jsonl`
   - Codex: `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`
   - Cursor: per-workspace `state.vscdb` SQLite (key `composerData:*`)
   - Aider: `<repo>/.aider.chat.history.md` (markdown with `> ` user prefix
     and ```diff fences)

2. **Implement `discover()`** — walk the platform-default directory and
   emit `SessionRef`s.

3. **Implement `parse()`** — read one source file / DB row, produce a
   [`NormalizedSession`](crates/drift-core/src/model.rs):
   - `session_id`: stable id from the agent (file stem, composer id, etc).
     If the agent has no stable id (e.g. Aider markdown), synthesise one
     from `sha256(path + start_timestamp)`.
   - `agent_slug`: a new variant of [`AgentSlug`](crates/drift-core/src/model.rs).
     Add the variant + the slug string in `as_str()` / `parse()` first.
   - `turns[]`: one per message. Each user / assistant exchange = one Turn.
     Put tool invocations into `Turn::tool_calls`, results into
     `Turn::tool_results`.
   - Timestamps, model, working_dir where available.

4. **Implement `extract_code_events()`** — walk the turns and emit a
   [`CodeEventDraft`](crates/drift-core/src/attribution.rs) for every
   file mutation. Re-use `drift_core::shell_lexer` for shell-command
   intent detection (`mv`, `cp`, `rm`, redirects, `sed -i`, best-effort
   `python -c open()`).

   For agents whose session format **doesn't** carry a tool-call /
   tool-result structure (Aider's markdown is the canonical example),
   default `rejected = false` and rely on the SHA-256 ladder in
   `drift_core::attribution` to catch divergence. Document this with a
   `[BEST-EFFORT]` comment so future readers know what's approximate.

5. **Add the feature flag** in `crates/drift-connectors/Cargo.toml`:
   ```toml
   [features]
   default = ["claude-code", "codex", "cursor", "aider", "your-agent"]
   your-agent = []
   ```
   Use `optional = true` on the dependency line if your agent's parser
   needs a heavyweight dep (rusqlite, etc) you don't want pulled in by
   default.

6. **Wire the connector** into `default_connectors()` in `lib.rs` under
   the feature gate.

7. **Add `--to your-agent`** to [`TargetAgent`](crates/drift-core/src/handoff.rs)
   if the agent is something a `drift handoff` brief can be pasted into.
   Add a footer style in `render_brief()` matching the agent's UX.

8. **Add tests with synthetic fixtures.** Don't commit real user data.
   The Cursor connector's `build_fixture_db` and Aider's inline
   `FIXTURE` constant show two patterns. Aim for ≥5 unit tests covering
   discover / parse / extract / edge cases / op inference.

9. **Add a real-API smoke** if your agent has a remote API (most don't —
   most write local files). Use the `#[ignore]` + env-gate pattern from
   `crates/drift-core/tests/v030_real_smoke.rs` so CI doesn't try to hit
   real endpoints.

## Attribution rules

- **Don't string-hack into columns that don't belong to them.** If you
  feel the urge, the data model is wrong — propose a change to
  PROPOSAL §D instead.
- **Don't claim authorship.** The `human` slug means "no AI session
  produced this", nothing more. That is the only honest claim we can
  make.
- **Rejected signal is primary.** If an agent's `tool_result` /
  `function_call_output` reports failure, the event must be emitted
  with `rejected = true` — not skipped.

## Code style

- `cargo fmt --all` before committing.
- `cargo clippy --all-targets --all-features -- -D warnings` stays
  green.
- Public API needs a doc comment. Non-obvious logic needs a short
  comment explaining the *why*.
- Conventional Commits for commit messages (`feat:`, `fix:`, `docs:`,
  `refactor:`, `test:`, `chore:`, `build:`, `ci:`).

## Security

Secrets come from environment only. Never commit `.env`, never read
from a config file for keys. If you see a hardcoded secret, open an
issue — security reports welcome as private GitHub advisories or
direct email to the maintainer listed in Cargo.toml.

## Running the full verification suite

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
```

All three must be green before a PR merges.
