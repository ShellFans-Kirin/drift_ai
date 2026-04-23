# drift-ai

The CLI binary for [**Drift AI**](https://github.com/ShellFans-Kirin/drift_ai) —
AI-native blame for the post-prompt era.

`drift` watches the local session logs of your AI coding agents
(Claude Code, Codex, Aider…), LLM-compacts each session, stores the
result in `.prompts/` inside your git repo, and builds a line-level
attribution layer that links every code event back to its originating
prompt. It also binds each session to its matching commit via
`refs/notes/drift`.

## Install

```bash
brew install ShellFans-Kirin/drift/drift        # macOS / Linux, any arch
cargo install drift-ai                          # if you have a Rust toolchain
```

(The crate publishes as `drift-ai` but installs the `drift` binary.)

## Quickstart

```bash
cd your-git-repo
drift init                                      # scaffold .prompts/
drift watch &                                   # event-driven capture in the background
drift blame src/foo.rs                          # reverse lookup: who wrote what
drift log                                       # per-commit agent attribution
drift cost --by session                         # per-session Anthropic spend
```

See the [top-level README](https://github.com/ShellFans-Kirin/drift_ai)
for the full command reference, MCP integration, and
cost / event-driven / context-window behaviour.

Licensed under [Apache-2.0](https://github.com/ShellFans-Kirin/drift_ai/blob/main/LICENSE).
