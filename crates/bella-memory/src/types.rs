use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One stored unit of episodic memory: a discrete interaction or event the
/// user had with the system (Phase 8's "episodic memory" — what happened,
/// when, and what it was about). This is deliberately simple for the v1
/// Memory Engine: no embeddings, no ranking yet, just durable structured
/// storage that later work (semantic search, summarization, forgetting)
/// builds on top of rather than around.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: Uuid,
    pub correlation_id: Uuid,
    /// Unix seconds. Stored as an integer column, not a string, so range
    /// queries and ordering stay index-friendly as the table grows.
    pub created_at_unix: i64,
    /// Free-text summary of what happened in this episode. In a fuller
    /// build this would be an LLM-generated summary (Phase 8 "automatic
    /// summarization"); for now it's the raw utterance/content passed in.
    pub content: String,
    /// Loosely-typed tag for filtering (e.g. "utterance", "action_taken").
    /// A real semantic/knowledge-graph layer builds on top of this, not
    /// instead of it — most retrieval will still want "recent episodes of
    /// this kind" as a cheap first filter before anything fancier runs.
    pub kind: String,
}

/// A new episode ready to be persisted — no id/timestamp yet, those are
/// assigned by the store at write time so callers can't accidentally
/// collide ids or backdate entries.
#[derive(Debug, Clone)]
pub struct NewEpisode {
    pub correlation_id: Uuid,
    pub content: String,
    pub kind: String,
}
