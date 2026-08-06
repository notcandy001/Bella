use crate::error::{BellaError, BellaResult};
use crate::message::Envelope;
use crate::subsystem::SubsystemId;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// Bounded channel capacity per subsystem inbox. Bounded, not unbounded —
/// an unbounded channel would let a stuck subsystem silently accumulate
/// unlimited memory. Backpressure (ChannelFull) is a real, handled error.
const INBOX_CAPACITY: usize = 256;

/// The internal message bus: the concrete implementation of Phase 2's
/// "subsystems communicate via async message passing over channels, not
/// direct function calls" decision. Subsystems register once at startup to
/// obtain a Receiver, and hold only a `MessageBus` handle (never each
/// other's structs) to send.
#[derive(Clone)]
pub struct MessageBus {
    senders: Arc<RwLock<HashMap<SubsystemId, mpsc::Sender<Envelope>>>>,
}

impl MessageBus {
    pub fn new() -> Self {
        Self {
            senders: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Called once by the supervisor when spawning a subsystem task.
    /// Returns the Receiver half; the bus keeps only the Sender half so
    /// every other subsystem can address this one, but nothing outside
    /// the owning task can ever pull messages meant for it.
    pub async fn register(&self, id: SubsystemId) -> mpsc::Receiver<Envelope> {
        let (tx, rx) = mpsc::channel(INBOX_CAPACITY);
        self.senders.write().await.insert(id, tx);
        rx
    }

    pub async fn deregister(&self, id: SubsystemId) {
        self.senders.write().await.remove(&id);
    }

    /// Send a message to a specific subsystem's inbox. Non-blocking with
    /// respect to correctness: if the inbox is full this returns
    /// ChannelFull rather than deadlocking the caller, so a slow
    /// subsystem degrades gracefully instead of stalling the whole daemon.
    pub async fn send(&self, envelope: Envelope) -> BellaResult<()> {
        let dest = envelope.destination;
        let senders = self.senders.read().await;
        let tx = senders
            .get(&dest)
            .ok_or_else(|| BellaError::SubsystemNotFound(dest.to_string()))?;

        match tx.try_send(envelope) {
            Ok(()) => Ok(()),
            Err(mpsc::error::TrySendError::Full(_)) => {
                Err(BellaError::ChannelFull(dest.to_string()))
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                Err(BellaError::ChannelClosed(dest.to_string()))
            }
        }
    }

    /// Broadcast a message to every currently-registered subsystem except
    /// the sender. Used for lifecycle events (e.g. ShuttingDown) where
    /// every subsystem needs to react, not one specific destination.
    pub async fn broadcast_except(&self, exclude: SubsystemId, make_envelope: impl Fn(SubsystemId) -> Envelope) {
        let senders = self.senders.read().await;
        for (&id, tx) in senders.iter() {
            if id == exclude {
                continue;
            }
            let _ = tx.try_send(make_envelope(id));
        }
    }

    pub async fn is_registered(&self, id: SubsystemId) -> bool {
        self.senders.read().await.contains_key(&id)
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{Envelope, Payload};
    use uuid::Uuid;

    #[tokio::test]
    async fn send_delivers_to_registered_subsystem() {
        let bus = MessageBus::new();
        let mut rx = bus.register(SubsystemId::Voice).await;

        let envelope = Envelope::new_root(
            SubsystemId::Core,
            SubsystemId::Voice,
            Payload::UserUtterance {
                text: "hello".into(),
            },
        );
        bus.send(envelope).await.expect("send should succeed");

        let received = rx.recv().await.expect("should receive message");
        match received.payload {
            Payload::UserUtterance { text } => assert_eq!(text, "hello"),
            _ => panic!("wrong payload variant"),
        }
    }

    #[tokio::test]
    async fn send_to_unregistered_subsystem_errors() {
        let bus = MessageBus::new();
        let envelope = Envelope::new_root(
            SubsystemId::Core,
            SubsystemId::Vision,
            Payload::Lifecycle(crate::message::LifecycleEvent::Started),
        );
        let result = bus.send(envelope).await;
        assert!(matches!(result, Err(BellaError::SubsystemNotFound(_))));
    }

    #[tokio::test]
    async fn full_inbox_returns_backpressure_error_not_deadlock() {
        let bus = MessageBus::new();
        let _rx = bus.register(SubsystemId::Memory).await; // never drained

        let mut last_result = Ok(());
        for _ in 0..(INBOX_CAPACITY + 10) {
            let envelope = Envelope::new_root(
                SubsystemId::Core,
                SubsystemId::Memory,
                Payload::Lifecycle(crate::message::LifecycleEvent::Ready),
            );
            last_result = bus.send(envelope).await;
        }
        assert!(matches!(last_result, Err(BellaError::ChannelFull(_))));
    }

    #[tokio::test]
    async fn correlation_id_propagates_across_a_chain() {
        let root = Envelope::new_root(
            SubsystemId::Voice,
            SubsystemId::Context,
            Payload::UserUtterance { text: "hi".into() },
        );
        let corr = root.correlation_id;
        let next = Envelope::new(
            SubsystemId::Context,
            SubsystemId::Reasoning,
            corr,
            Payload::AssembledContext {
                context_json: "{}".into(),
            },
        );
        assert_eq!(root.correlation_id, next.correlation_id);
        assert_ne!(root.id, next.id);
        let _ = Uuid::new_v4(); // sanity: uuid crate wired correctly
    }
}
