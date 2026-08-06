//! bella-memory: the Memory Engine. SQLite-backed episodic memory, wrapped
//! in an async-safe API. Per the Phase 2 dependency graph, the Context
//! Builder depends on this crate to fold real history into prompt
//! context — this is what upgrades Context from "logs whatever it's told"
//! to "actually remembers."

pub mod engine;
pub mod store;
pub mod types;

pub use engine::MemoryEngine;
pub use types::{Episode, NewEpisode};
