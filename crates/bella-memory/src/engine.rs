use crate::store::MemoryStore;
use crate::types::{Episode, NewEpisode};
use bella_common::{BellaError, BellaResult};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// The Memory Engine's public, async-friendly interface. `rusqlite` is
/// synchronous and holds a blocking lock on the file, so every call here
/// is dispatched onto tokio's blocking thread pool via `spawn_blocking` —
/// this is the load-bearing detail that keeps a slow disk write from ever
/// stalling the async reactor that the rest of the daemon (voice, bus)
/// runs on.
#[derive(Clone)]
pub struct MemoryEngine {
    store: Arc<Mutex<MemoryStore>>,
}

impl MemoryEngine {
    pub fn open(db_path: &str) -> BellaResult<Self> {
        let store = MemoryStore::open(db_path)
            .map_err(|e| BellaError::Internal(format!("failed to open memory db: {e}")))?;
        Ok(Self {
            store: Arc::new(Mutex::new(store)),
        })
    }

    #[cfg(test)]
    pub fn open_in_memory() -> BellaResult<Self> {
        let store = MemoryStore::open_in_memory()
            .map_err(|e| BellaError::Internal(format!("failed to open memory db: {e}")))?;
        Ok(Self {
            store: Arc::new(Mutex::new(store)),
        })
    }

    pub async fn record(&self, new: NewEpisode) -> BellaResult<Episode> {
        let store = self.store.clone();
        tokio::task::spawn_blocking(move || {
            let guard = store.lock().expect("memory store mutex poisoned");
            guard.insert_episode(new)
        })
        .await
        .map_err(|e| BellaError::Internal(format!("memory task join error: {e}")))?
        .map_err(|e| BellaError::Internal(format!("memory insert failed: {e}")))
    }

    pub async fn recent(&self, limit: u32) -> BellaResult<Vec<Episode>> {
        let store = self.store.clone();
        tokio::task::spawn_blocking(move || {
            let guard = store.lock().expect("memory store mutex poisoned");
            guard.recent_episodes(limit)
        })
        .await
        .map_err(|e| BellaError::Internal(format!("memory task join error: {e}")))?
        .map_err(|e| BellaError::Internal(format!("memory query failed: {e}")))
    }

    pub async fn interaction_history(&self, correlation_id: Uuid) -> BellaResult<Vec<Episode>> {
        let store = self.store.clone();
        tokio::task::spawn_blocking(move || {
            let guard = store.lock().expect("memory store mutex poisoned");
            guard.episodes_by_correlation(correlation_id)
        })
        .await
        .map_err(|e| BellaError::Internal(format!("memory task join error: {e}")))?
        .map_err(|e| BellaError::Internal(format!("memory query failed: {e}")))
    }

    /// Placeholder-but-real substring search — see the comment on
    /// `MemoryStore::search_content` for why this isn't semantic search yet.
    pub async fn search(&self, query: String, limit: u32) -> BellaResult<Vec<Episode>> {
        let store = self.store.clone();
        tokio::task::spawn_blocking(move || {
            let guard = store.lock().expect("memory store mutex poisoned");
            guard.search_content(&query, limit)
        })
        .await
        .map_err(|e| BellaError::Internal(format!("memory task join error: {e}")))?
        .map_err(|e| BellaError::Internal(format!("memory query failed: {e}")))
    }

    pub async fn count(&self) -> BellaResult<u64> {
        let store = self.store.clone();
        tokio::task::spawn_blocking(move || {
            let guard = store.lock().expect("memory store mutex poisoned");
            guard.count()
        })
        .await
        .map_err(|e| BellaError::Internal(format!("memory task join error: {e}")))?
        .map_err(|e| BellaError::Internal(format!("memory query failed: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::NewEpisode;

    #[tokio::test]
    async fn record_then_recall_round_trips() {
        let engine = MemoryEngine::open_in_memory().unwrap();
        let corr = Uuid::new_v4();
        engine
            .record(NewEpisode {
                correlation_id: corr,
                content: "user asked about their calendar".into(),
                kind: "utterance".into(),
            })
            .await
            .unwrap();

        let recent = engine.recent(10).await.unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].content, "user asked about their calendar");
        assert_eq!(recent[0].correlation_id, corr);
    }

    #[tokio::test]
    async fn recent_respects_limit_and_ordering() {
        let engine = MemoryEngine::open_in_memory().unwrap();
        for i in 0..5 {
            engine
                .record(NewEpisode {
                    correlation_id: Uuid::new_v4(),
                    content: format!("episode {i}"),
                    kind: "utterance".into(),
                })
                .await
                .unwrap();
        }
        let recent = engine.recent(3).await.unwrap();
        assert_eq!(recent.len(), 3);
        // Most recent (highest index inserted last) should come first.
        assert_eq!(recent[0].content, "episode 4");
    }

    #[tokio::test]
    async fn interaction_history_groups_by_correlation_in_order() {
        let engine = MemoryEngine::open_in_memory().unwrap();
        let corr = Uuid::new_v4();
        engine
            .record(NewEpisode {
                correlation_id: corr,
                content: "step one".into(),
                kind: "utterance".into(),
            })
            .await
            .unwrap();
        engine
            .record(NewEpisode {
                correlation_id: corr,
                content: "step two".into(),
                kind: "action_taken".into(),
            })
            .await
            .unwrap();
        engine
            .record(NewEpisode {
                correlation_id: Uuid::new_v4(),
                content: "unrelated episode".into(),
                kind: "utterance".into(),
            })
            .await
            .unwrap();

        let history = engine.interaction_history(corr).await.unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].content, "step one");
        assert_eq!(history[1].content, "step two");
    }

    #[tokio::test]
    async fn search_finds_substring_matches_only() {
        let engine = MemoryEngine::open_in_memory().unwrap();
        engine
            .record(NewEpisode {
                correlation_id: Uuid::new_v4(),
                content: "remind me to buy groceries".into(),
                kind: "utterance".into(),
            })
            .await
            .unwrap();
        engine
            .record(NewEpisode {
                correlation_id: Uuid::new_v4(),
                content: "what's the weather like".into(),
                kind: "utterance".into(),
            })
            .await
            .unwrap();

        let results = engine.search("groceries".into(), 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("groceries"));
    }

    #[tokio::test]
    async fn count_reflects_number_of_stored_episodes() {
        let engine = MemoryEngine::open_in_memory().unwrap();
        assert_eq!(engine.count().await.unwrap(), 0);
        engine
            .record(NewEpisode {
                correlation_id: Uuid::new_v4(),
                content: "one".into(),
                kind: "utterance".into(),
            })
            .await
            .unwrap();
        assert_eq!(engine.count().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn persists_across_engine_instances_on_same_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("memory.db");
        let path_str = path.to_str().unwrap();

        {
            let engine = MemoryEngine::open(path_str).unwrap();
            engine
                .record(NewEpisode {
                    correlation_id: Uuid::new_v4(),
                    content: "persisted episode".into(),
                    kind: "utterance".into(),
                })
                .await
                .unwrap();
        } // engine dropped, connection closed

        let reopened = MemoryEngine::open(path_str).unwrap();
        let recent = reopened.recent(10).await.unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].content, "persisted episode");
    }
}
