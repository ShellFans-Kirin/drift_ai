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

The worked example is Aider
([`crates/drift-connectors/src/aider.rs`](crates/drift-connectors/src/aider.rs)).
The steps to turn the stub into a real connector:

1. Find the on-disk session layout. For Aider, sessions live under
   `~/.aider/` with a JSON-lines history. Capture a real sample first
   (`~/.aider/chat-history.jsonl` in most installs).
2. Implement `discover()` — walk the dir, return `.jsonl` paths.
3. Implement `parse()` — read the file, produce a `NormalizedSession`:
   - `session_id`: whatever the agent calls it (file stem is fine)
   - `agent_slug`: `AgentSlug::Aider`
   - `turns[]`: one per message; put tool calls into `Turn::tool_calls`
   - timestamps, model, working_dir where available
4. Implement `extract_code_events()` — walk the turns and emit
   `CodeEventDraft`s for every file mutation. Reuse
   `drift_core::shell_lexer` for shell-command intent detection; it
   handles `mv`, `cp`, `rm`, redirects, `sed -i`, best-effort
   `python -c open()`.
5. Add the feature flag in `crates/drift-connectors/Cargo.toml`:
   ```toml
   [features]
   aider = []
   ```
6. Wire the connector into `default_connectors()` in `lib.rs` under
   the feature gate.
7. Drop 3 fixture `.jsonl`s into `tests/fixtures/aider/` (plain chat,
   edit cycle, failed-retry) and add a `connector_aider.rs`
   integration test.

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
