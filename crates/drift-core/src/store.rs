//! SQLite-backed store for [`CodeEvent`] and compacted session metadata.
//!
//! The on-disk schema intentionally tracks the proposal schema (§D.2) 1:1.
//! The store is the only writer; readers use the query helpers here or
//! directly (for `drift blame` / `drift trace`).

use crate::model::{AgentSlug, CodeEvent, Operation};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension as _, Row};
use std::path::Path;

/// SQLite `application_id` (PRAGMA). Picked so `file` sniffs identify our DBs.
/// Derived from ASCII "drft" interpreted as LE u32.
pub const APPLICATION_ID: i32 = 0x64_72_66_74; // "drft"

const SCHEMA_V1: &str = r#"
CREATE TABLE IF NOT EXISTS code_events (
    event_id              TEXT PRIMARY KEY,
    session_id            TEXT,
    agent_slug            TEXT NOT NULL,
    turn_id               TEXT,
    timestamp             TEXT NOT NULL,    -- ISO 8601 UTC
    file_path             TEXT NOT NULL,
    operation             TEXT NOT NULL,
    rename_from           TEXT,
    line_ranges_before    TEXT NOT NULL,    -- JSON array of [start,end]
    line_ranges_after     TEXT NOT NULL,    -- JSON array of [start,end]
    diff_hunks            TEXT NOT NULL,
    rejected              INTEGER NOT NULL DEFAULT 0,
    parent_event_id       TEXT,
    content_sha256_after  TEXT,
    bound_commit_sha      TEXT,
    metadata              TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS idx_events_file_ts    ON code_events(file_path, timestamp);
CREATE INDEX IF NOT EXISTS idx_events_session    ON code_events(session_id);
CREATE INDEX IF NOT EXISTS idx_events_commit     ON code_events(bound_commit_sha);
CREATE INDEX IF NOT EXISTS idx_events_rejected   ON code_events(rejected) WHERE rejected = 1;
CREATE INDEX IF NOT EXISTS idx_events_parent     ON code_events(parent_event_id);

CREATE TABLE IF NOT EXISTS sessions (
    session_id    TEXT PRIMARY KEY,
    agent_slug    TEXT NOT NULL,
    model         TEXT,
    working_dir   TEXT,
    started_at    TEXT NOT NULL,
    ended_at      TEXT NOT NULL,
    turn_count    INTEGER NOT NULL DEFAULT 0,
    thinking_blocks INTEGER NOT NULL DEFAULT 0,
    compacted_md_path TEXT,
    summary       TEXT NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS idx_sessions_agent    ON sessions(agent_slug);
CREATE INDEX IF NOT EXISTS idx_sessions_started  ON sessions(started_at);

CREATE TABLE IF NOT EXISTS file_shas (
    file_path         TEXT PRIMARY KEY,
    last_known_sha256 TEXT NOT NULL,
    last_event_id     TEXT NOT NULL,
    updated_at        TEXT NOT NULL
);
"#;

pub struct EventStore {
    pub(crate) conn: Connection,
}

impl EventStore {
    /// Open (or create) the SQLite DB at `path`, applying migrations.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path.as_ref())
            .with_context(|| format!("open events.db at {}", path.as_ref().display()))?;
        Self::init(&conn)?;
        Ok(Self { conn })
    }

    /// In-memory store, for tests and dry-runs.
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::init(&conn)?;
        Ok(Self { conn })
    }

    fn init(conn: &Connection) -> Result<()> {
        conn.pragma_update(None, "application_id", APPLICATION_ID)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.execute_batch(SCHEMA_V1)?;
        Ok(())
    }

    /// Idempotent upsert of a code event.
    pub fn insert_event(&self, ev: &CodeEvent) -> Result<()> {
        let lrb = serde_json::to_string(&ev.line_ranges_before)?;
        let lra = serde_json::to_string(&ev.line_ranges_after)?;
        let md = serde_json::to_string(&ev.metadata)?;
        self.conn.execute(
            r#"INSERT OR REPLACE INTO code_events
                (event_id, session_id, agent_slug, turn_id, timestamp, file_path,
                 operation, rename_from, line_ranges_before, line_ranges_after,
                 diff_hunks, rejected, parent_event_id, content_sha256_after,
                 bound_commit_sha, metadata)
               VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)"#,
            params![
                ev.event_id,
                ev.session_id,
                ev.agent_slug.as_str(),
                ev.turn_id,
                ev.timestamp.to_rfc3339(),
                ev.file_path,
                ev.operation.as_str(),
                ev.rename_from,
                lrb,
                lra,
                ev.diff_hunks,
                ev.rejected as i64,
                ev.parent_event_id,
                ev.content_sha256_after,
                ev.bound_commit_sha,
                md,
            ],
        )?;

        // Maintain the file_shas ladder (PROPOSAL §D.3).
        if !ev.rejected {
            if let Some(sha) = &ev.content_sha256_after {
                self.conn.execute(
                    r#"INSERT INTO file_shas (file_path, last_known_sha256, last_event_id, updated_at)
                       VALUES (?1, ?2, ?3, ?4)
                       ON CONFLICT(file_path) DO UPDATE SET
                           last_known_sha256 = excluded.last_known_sha256,
                           last_event_id = excluded.last_event_id,
                           updated_at = excluded.updated_at"#,
                    params![ev.file_path, sha, ev.event_id, ev.timestamp.to_rfc3339()],
                )?;
            }
        }
        Ok(())
    }

    pub fn last_known_sha(&self, file_path: &str) -> Result<Option<(String, String)>> {
        Ok(self
            .conn
            .query_row(
                "SELECT last_known_sha256, last_event_id FROM file_shas WHERE file_path = ?1",
                [file_path],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .optional()?)
    }

    pub fn insert_session_meta(
        &self,
        session_id: &str,
        agent_slug: AgentSlug,
        model: Option<&str>,
        working_dir: Option<&str>,
        started_at: DateTime<Utc>,
        ended_at: DateTime<Utc>,
        turn_count: u32,
        thinking_blocks: u32,
        compacted_md_path: Option<&str>,
        summary: &str,
    ) -> Result<()> {
        self.conn.execute(
            r#"INSERT OR REPLACE INTO sessions
                (session_id, agent_slug, model, working_dir, started_at, ended_at,
                 turn_count, thinking_blocks, compacted_md_path, summary)
               VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)"#,
            params![
                session_id,
                agent_slug.as_str(),
                model,
                working_dir,
                started_at.to_rfc3339(),
                ended_at.to_rfc3339(),
                turn_count,
                thinking_blocks,
                compacted_md_path,
                summary,
            ],
        )?;
        Ok(())
    }

    /// Events on a given file, oldest → newest.
    pub fn events_for_file(&self, file_path: &str) -> Result<Vec<CodeEvent>> {
        let mut stmt = self.conn.prepare(
            r#"SELECT * FROM code_events WHERE file_path = ?1 ORDER BY timestamp ASC"#,
        )?;
        let rows = stmt.query_map([file_path], row_to_event)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Events produced by a session, oldest → newest.
    pub fn events_for_session(&self, session_id: &str) -> Result<Vec<CodeEvent>> {
        let mut stmt = self.conn.prepare(
            r#"SELECT * FROM code_events WHERE session_id = ?1 ORDER BY timestamp ASC"#,
        )?;
        let rows = stmt.query_map([session_id], row_to_event)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn rejected_events(&self, since: Option<&str>) -> Result<Vec<CodeEvent>> {
        let mut out = Vec::new();
        match since {
            Some(ts) => {
                let mut stmt = self.conn.prepare(
                    "SELECT * FROM code_events WHERE rejected = 1 AND timestamp >= ?1 \
                     ORDER BY timestamp ASC",
                )?;
                let rows = stmt.query_map([ts], row_to_event)?;
                for r in rows {
                    out.push(r?);
                }
            }
            None => {
                let mut stmt = self.conn.prepare(
                    "SELECT * FROM code_events WHERE rejected = 1 ORDER BY timestamp ASC",
                )?;
                let rows = stmt.query_map([], row_to_event)?;
                for r in rows {
                    out.push(r?);
                }
            }
        }
        Ok(out)
    }

    pub fn event_by_id(&self, event_id: &str) -> Result<Option<CodeEvent>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM code_events WHERE event_id = ?1")?;
        let mut rows = stmt.query([event_id])?;
        if let Some(r) = rows.next()? {
            Ok(Some(row_to_event(r)?))
        } else {
            Ok(None)
        }
    }

    pub fn bind_commit(&self, event_id: &str, commit_sha: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE code_events SET bound_commit_sha = ?1 WHERE event_id = ?2",
            params![commit_sha, event_id],
        )?;
        Ok(())
    }

    pub fn sessions_in_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<SessionRow>> {
        let mut stmt = self.conn.prepare(
            r#"SELECT session_id, agent_slug, model, started_at, ended_at, turn_count, summary
               FROM sessions
               WHERE started_at >= ?1 AND ended_at <= ?2
               ORDER BY started_at ASC"#,
        )?;
        let rows = stmt.query_map(
            params![start.to_rfc3339(), end.to_rfc3339()],
            |r| {
                Ok(SessionRow {
                    session_id: r.get(0)?,
                    agent_slug: AgentSlug::parse(&r.get::<_, String>(1)?),
                    model: r.get::<_, Option<String>>(2)?,
                    started_at: parse_ts(&r.get::<_, String>(3)?),
                    ended_at: parse_ts(&r.get::<_, String>(4)?),
                    turn_count: r.get::<_, i64>(5)? as u32,
                    summary: r.get(6)?,
                })
            },
        )?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn events_for_commit(&self, commit_sha: &str) -> Result<Vec<CodeEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM code_events WHERE bound_commit_sha = ?1 ORDER BY timestamp ASC",
        )?;
        let rows = stmt.query_map([commit_sha], row_to_event)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub fn list_sessions(&self, agent: Option<AgentSlug>) -> Result<Vec<SessionRow>> {
        let mut out = Vec::new();
        let mapper = |r: &Row| {
            Ok(SessionRow {
                session_id: r.get(0)?,
                agent_slug: AgentSlug::parse(&r.get::<_, String>(1)?),
                model: r.get::<_, Option<String>>(2)?,
                started_at: parse_ts(&r.get::<_, String>(3)?),
                ended_at: parse_ts(&r.get::<_, String>(4)?),
                turn_count: r.get::<_, i64>(5)? as u32,
                summary: r.get(6)?,
            })
        };
        match agent {
            Some(a) => {
                let slug = a.as_str();
                let mut stmt = self.conn.prepare(
                    "SELECT session_id, agent_slug, model, started_at, ended_at, turn_count, summary \
                     FROM sessions WHERE agent_slug = ?1 ORDER BY started_at DESC",
                )?;
                let rows = stmt.query_map([slug], mapper)?;
                for r in rows {
                    out.push(r?);
                }
            }
            None => {
                let mut stmt = self.conn.prepare(
                    "SELECT session_id, agent_slug, model, started_at, ended_at, turn_count, summary \
                     FROM sessions ORDER BY started_at DESC",
                )?;
                let rows = stmt.query_map([], mapper)?;
                for r in rows {
                    out.push(r?);
                }
            }
        }
        Ok(out)
    }
}

#[derive(Debug, Clone)]
pub struct SessionRow {
    pub session_id: String,
    pub agent_slug: AgentSlug,
    pub model: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub turn_count: u32,
    pub summary: String,
}

fn parse_ts(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn row_to_event(r: &Row) -> rusqlite::Result<CodeEvent> {
    let line_ranges_before_s: String = r.get("line_ranges_before")?;
    let line_ranges_after_s: String = r.get("line_ranges_after")?;
    let md_s: String = r.get("metadata")?;
    let timestamp_s: String = r.get("timestamp")?;
    let agent_s: String = r.get("agent_slug")?;
    let op_s: String = r.get("operation")?;
    Ok(CodeEvent {
        event_id: r.get("event_id")?,
        session_id: r.get("session_id")?,
        agent_slug: AgentSlug::parse(&agent_s),
        turn_id: r.get("turn_id")?,
        timestamp: parse_ts(&timestamp_s),
        file_path: r.get("file_path")?,
        operation: Operation::parse(&op_s).unwrap_or(Operation::Edit),
        rename_from: r.get("rename_from")?,
        line_ranges_before: serde_json::from_str(&line_ranges_before_s).unwrap_or_default(),
        line_ranges_after: serde_json::from_str(&line_ranges_after_s).unwrap_or_default(),
        diff_hunks: r.get("diff_hunks")?,
        rejected: r.get::<_, i64>("rejected")? != 0,
        parent_event_id: r.get("parent_event_id")?,
        content_sha256_after: r.get("content_sha256_after")?,
        bound_commit_sha: r.get("bound_commit_sha")?,
        metadata: serde_json::from_str(&md_s).unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AgentSlug, Operation};
    use chrono::TimeZone;

    fn mk_event(file: &str, sha: &str) -> CodeEvent {
        CodeEvent {
            event_id: CodeEvent::new_id(),
            session_id: Some("sess-1".into()),
            agent_slug: AgentSlug::ClaudeCode,
            turn_id: Some("t1".into()),
            timestamp: Utc.with_ymd_and_hms(2026, 4, 22, 10, 0, 0).unwrap(),
            file_path: file.into(),
            operation: Operation::Edit,
            rename_from: None,
            line_ranges_before: vec![(1, 1)],
            line_ranges_after: vec![(1, 2)],
            diff_hunks: "--- a\n+++ b\n@@\n+hello\n".into(),
            rejected: false,
            parent_event_id: None,
            content_sha256_after: Some(sha.into()),
            bound_commit_sha: None,
            metadata: Default::default(),
        }
    }

    #[test]
    fn roundtrip_and_sha_ladder() {
        let s = EventStore::open_in_memory().unwrap();
        let e = mk_event("src/main.rs", "abc123");
        s.insert_event(&e).unwrap();
        let got = s.event_by_id(&e.event_id).unwrap().unwrap();
        assert_eq!(got.file_path, "src/main.rs");
        assert_eq!(
            s.last_known_sha("src/main.rs").unwrap().unwrap().0,
            "abc123"
        );
    }
}
