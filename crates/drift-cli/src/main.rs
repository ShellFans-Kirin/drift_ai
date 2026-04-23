//! `drift` — the CLI binary.
//!
//! Subcommand layout mirrors PROPOSAL §E.4, plus `drift mcp` for the MCP
//! server. Every subcommand is a thin shell; heavy lifting lives in
//! `drift-core` / `drift-connectors` / `drift-mcp`.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod commands;

#[derive(Parser)]
#[command(
    name = "drift",
    version,
    about = "Drift AI — capture, compact, and bind AI coding sessions to your git history."
)]
struct Cli {
    /// Override the repo root (defaults to CWD).
    #[arg(long, global = true)]
    repo: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Scaffold .prompts/ and write project config.
    Init,

    /// One-shot pull of session(s) from the configured connectors.
    Capture {
        #[arg(long)]
        session: Option<String>,
        #[arg(long)]
        agent: Option<String>,
        #[arg(long = "all-since")]
        all_since: Option<String>,
    },

    /// Background daemon monitoring both agent directories.
    Watch,

    /// Aggregate compaction token usage and cost.
    Cost {
        /// ISO-8601 lower bound for called_at (inclusive).
        #[arg(long)]
        since: Option<String>,
        /// ISO-8601 upper bound for called_at (inclusive).
        #[arg(long)]
        until: Option<String>,
        /// Exact model string to filter on.
        #[arg(long)]
        model: Option<String>,
        /// Group output by `model`, `session`, or `date`.
        #[arg(long = "by")]
        by: Option<String>,
    },

    /// List captured sessions.
    List {
        #[arg(long)]
        agent: Option<String>,
    },

    /// Render a single compacted session.
    Show { session_id: String },

    /// Reverse lookup: line -> CodeEvent timeline (the blame view).
    Blame {
        file: PathBuf,
        #[arg(long)]
        line: Option<u32>,
        #[arg(long)]
        range: Option<String>,
    },

    /// Forward lookup: session -> all events it produced.
    Trace { session_id: String },

    /// Render a single event's diff.
    Diff { event_id: String },

    /// List rejected AI suggestions.
    Rejected {
        #[arg(long)]
        since: Option<String>,
    },

    /// git log with per-commit session summaries merged in.
    Log {
        #[arg(last = true)]
        git_args: Vec<String>,
    },

    /// Get/set config values.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Manually bind a session to a commit.
    Bind { commit: String, session_id: String },

    /// Bind every captured session to its closest commit by timestamp.
    AutoBind,

    /// Install a post-commit git hook that runs `drift auto-bind`.
    InstallHook,

    /// Push/pull refs/notes/drift.
    Sync {
        #[command(subcommand)]
        dir: SyncDir,
    },

    /// Run the stdio MCP server (tools: drift_blame / drift_trace / drift_rejected / drift_log / drift_show_event).
    Mcp,
}

#[derive(Subcommand)]
enum ConfigAction {
    Get { key: String },
    Set { key: String, value: String },
    List,
}

#[derive(Subcommand)]
enum SyncDir {
    Push { remote: String },
    Pull { remote: String },
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn,drift=info")),
        )
        .init();

    let cli = Cli::parse();
    let repo = cli
        .repo
        .clone()
        .or_else(|| std::env::current_dir().ok())
        .context("resolve repo root")?;

    match cli.command {
        Command::Init => commands::init::run(&repo),
        Command::Capture {
            session,
            agent,
            all_since,
        } => commands::capture::run(
            &repo,
            session.as_deref(),
            agent.as_deref(),
            all_since.as_deref(),
        ),
        Command::Watch => commands::watch::run(&repo),
        Command::Cost {
            since,
            until,
            model,
            by,
        } => commands::cost::run(
            &repo,
            since.as_deref(),
            until.as_deref(),
            model.as_deref(),
            by.as_deref(),
        ),
        Command::List { agent } => commands::list::run(&repo, agent.as_deref()),
        Command::Show { session_id } => commands::show::run(&repo, &session_id),
        Command::Blame { file, line, range } => {
            commands::blame::run(&repo, &file, line, range.as_deref())
        }
        Command::Trace { session_id } => commands::trace::run(&repo, &session_id),
        Command::Diff { event_id } => commands::diff::run(&repo, &event_id),
        Command::Rejected { since } => commands::rejected::run(&repo, since.as_deref()),
        Command::Log { git_args } => commands::log::run(&repo, &git_args),
        Command::Config { action } => match action {
            ConfigAction::Get { key } => commands::config::get(&repo, &key),
            ConfigAction::Set { key, value } => commands::config::set(&repo, &key, &value),
            ConfigAction::List => commands::config::list(&repo),
        },
        Command::Bind { commit, session_id } => commands::bind::run(&repo, &commit, &session_id),
        Command::AutoBind => commands::auto_bind::run(&repo),
        Command::InstallHook => commands::install_hook::run(&repo),
        Command::Sync { dir } => match dir {
            SyncDir::Push { remote } => commands::sync::push(&repo, &remote),
            SyncDir::Pull { remote } => commands::sync::pull(&repo, &remote),
        },
        Command::Mcp => drift_mcp::run_stdio(&repo),
    }
}
