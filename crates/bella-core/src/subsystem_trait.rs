use async_trait::async_trait;
use tokio::sync::mpsc;
use bella_common::{Envelope, MessageBus, SubsystemId, BellaResult};

/// The interface every subsystem (Voice, Vision, Memory, Reasoning, ...)
/// implements. This is the Rust-trait realization of Phase 2's "every
/// subsystem must communicate through interfaces" rule: the supervisor
/// only ever knows about this trait, never about a concrete subsystem's
/// internals, which is exactly what makes subsystems independently
/// replaceable.
#[async_trait]
pub trait Subsystem: Send + 'static {
    fn id(&self) -> SubsystemId;

    /// Run the subsystem's main loop. Implementations should loop on
    /// `rx.recv()` and return only on a genuine, unrecoverable failure or
    /// a clean shutdown request — the supervisor treats any `Err` return
    /// (and any panic) as a signal to restart this subsystem.
    async fn run(&mut self, bus: MessageBus, rx: mpsc::Receiver<Envelope>) -> BellaResult<()>;
}
