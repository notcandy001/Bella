use crate::types::{Episode, NewEpisode};
use rusqlite::{params, Connection, OptionalExtension};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Thin synchronous wrapper around a SQLite connection. Kept deliberately
/// separate from `MemoryEngine` (in engine.rs): this struct owns all raw
/// SQL, and is the only place in the crate that does — everything else
/// talks to `MemoryStore` through plain Rust method calls, never SQL
/// strings. That boundary is what makes it possible to swap the storage
/// backend later (Phase 8 mentions a knowledge graph / vector DB
/// eventually) without touching the async engine or its callers.
pub struct MemoryStore {
    conn: Connection,
}

impl MemoryStore {
    /// Opens (creating if needed) the SQLite database at `path` and
    /// ensures the schema exists. `path` is caller-provided rather than
    /// hardcoded so tests can point at a temp file and production can
    /// point at the real per-user data directory.
    pub fn open(path: &str) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;
            CREATE TABLE IF NOT EXISTS episodes (
                id              TEXT PRIMARY KEY,
                correlation_id  TEXT NOT NULL,
                created_at_unix INTEGER NOT NULL,
                content         TEXT NOT NULL,
                kind            TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_episodes_created_at
                ON episodes (created_at_unix DESC);
            CREATE INDEX IF NOT EXISTS idx_episodes_correlation
                ON episodes (correlation_id);
            ",
        )?;
        Ok(Self { conn })
    }

    /// In-memory database, used only by tests that don't need to verify
    /// persistence-across-restart (that case uses a real temp file instead).
    #[cfg(test)]
    pub fn open_in_memory() -> rusqlite::Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "
            CREATE TABLE episodes (
                id              TEXT PRIMARY KEY,
                correlation_id  TEXT NOT NULL,
                created_at_unix INTEGER NOT NULL,
                content         TEXT NOT NULL,
                kind            TEXT NOT NULL
            );
            CREATE INDEX idx_episodes_created_at ON episodes (created_at_unix DESC);
            CREATE INDEX idx_episodes_correlation ON episodes (correlation_id);
            ",
        )?;
        Ok(Self { conn })
    }

    pub fn insert_episode(&self, new: NewEpisode) -> rusqlite::Result<Episode> {
        let id = Uuid::new_v4();
        let created_at_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_secs() as i64;

        self.conn.execute(
            "INSERT INTO episodes (id, correlation_id, created_at_unix, content, kind)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                id.to_string(),
                new.correlation_id.to_string(),
                created_at_unix,
                new.content,
                new.kind,
            ],
        )?;

        Ok(Episode {
            id,
            correlation_id: new.correlation_id,
            created_at_unix,
            content: new.content,
            kind: new.kind,
        })
    }

    /// Most-recent-first, capped at `limit`. This is the workhorse query
    /// the Context Builder uses to fold recent history into a prompt —
    /// deliberately simple (no relevance ranking yet) so Phase 8's
    /// "memory ranking" work has an honest, unranked baseline to improve on.
    pub fn recent_episodes(&self, limit: u32) -> rusqlite::Result<Vec<Episode>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, correlation_id, created_at_unix, content, kind
             FROM episodes
             ORDER BY created_at_unix DESC, rowid DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], row_to_episode)?;
        rows.collect()
    }

    /// All episodes sharing a correlation id, oldest first — reconstructs
    /// one full interaction chain (e.g. everything that happened for one
    /// voice query) in the order it actually occurred.
    pub fn episodes_by_correlation(&self, correlation_id: Uuid) -> rusqlite::Result<Vec<Episode>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, correlation_id, created_at_unix, content, kind
             FROM episodes
             WHERE correlation_id = ?1
             ORDER BY created_at_unix ASC, rowid ASC",
        )?;
        let rows = stmt.query_map(params![correlation_id.to_string()], row_to_episode)?;
        rows.collect()
    }

    /// Naive substring search over content. This is explicitly a
    /// placeholder for real semantic search (Phase 8: vector search over
    /// embeddings) — it's here so retrieval-by-topic works at all in v1,
    /// not because it's the intended long-term approach.
    pub fn search_content(&self, query: &str, limit: u32) -> rusqlite::Result<Vec<Episode>> {
        let pattern = format!("%{query}%");
        let mut stmt = self.conn.prepare(
            "SELECT id, correlation_id, created_at_unix, content, kind
             FROM episodes
             WHERE content LIKE ?1
             ORDER BY created_at_unix DESC, rowid DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![pattern, limit], row_to_episode)?;
        rows.collect()
    }

    pub fn get_by_id(&self, id: Uuid) -> rusqlite::Result<Option<Episode>> {
        self.conn
            .query_row(
                "SELECT id, correlation_id, created_at_unix, content, kind
                 FROM episodes WHERE id = ?1",
                params![id.to_string()],
                row_to_episode,
            )
            .optional()
    }

    pub fn count(&self) -> rusqlite::Result<u64> {
        self.conn
            .query_row("SELECT COUNT(*) FROM episodes", [], |row| row.get(0))
    }
}

fn row_to_episode(row: &rusqlite::Row) -> rusqlite::Result<Episode> {
    let id_str: String = row.get(0)?;
    let corr_str: String = row.get(1)?;
    Ok(Episode {
        id: Uuid::parse_str(&id_str).unwrap_or_else(|_| Uuid::nil()),
        correlation_id: Uuid::parse_str(&corr_str).unwrap_or_else(|_| Uuid::nil()),
        created_at_unix: row.get(2)?,
        content: row.get(3)?,
        kind: row.get(4)?,
    })
}
