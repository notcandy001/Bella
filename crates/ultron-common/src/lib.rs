//! ultron-common: shared types every subsystem depends on — the
//! foundational crate in the Phase 2 dependency graph. Nothing in this
//! crate depends on any other ultron-* crate; that's what makes it safe
//! for everything else to depend on it without creating cycles.

pub mod bus;
pub mod error;
pub mod message;
pub mod subsystem;

pub use bus::MessageBus;
pub use error::{UltronError, UltronResult};
pub use message::{Envelope, LifecycleEvent, Payload};
pub use subsystem::SubsystemId;
