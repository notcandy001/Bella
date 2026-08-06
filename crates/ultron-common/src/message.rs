use crate::subsystem::SubsystemId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Every message on the internal bus is wrapped in an Envelope. The
/// envelope carries routing/tracing metadata so the supervisor and audit
/// log can see *who* sent *what* to *whom* without inspecting payloads —
/// this is what makes the "recovery flow" and audit logging from Phase 1/2
/// possible without every subsystem re-implementing tracing itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    /// Unique id for this specific message instance.
    pub id: Uuid,
    /// Correlates a chain of messages belonging to one user interaction
    /// (e.g. one voice query -> context -> reasoning -> action -> reply).
    pub correlation_id: Uuid,
    pub source: SubsystemId,
    pub destination: SubsystemId,
    pub payload: Payload,
}

impl Envelope {
    pub fn new(
        source: SubsystemId,
        destination: SubsystemId,
        correlation_id: Uuid,
        payload: Payload,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            correlation_id,
            source,
            destination,
            payload,
        }
    }

    /// Start a new interaction chain (first message of a correlation group).
    pub fn new_root(source: SubsystemId, destination: SubsystemId, payload: Payload) -> Self {
        Self::new(source, destination, Uuid::new_v4(), payload)
    }
}

/// The set of message payloads subsystems can exchange. This enum is the
/// literal contract referenced in Phase 2: a subsystem is defined by which
/// of these variants it accepts and emits, not by its implementation.
/// New subsystems extend this enum; they never get a private side-channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Payload {
    /// Raw transcribed user utterance from the Voice subsystem.
    UserUtterance { text: String },
    /// Assembled prompt context, ready for the Reasoning Engine.
    AssembledContext { context_json: String },
    /// A proposed action coming back from the Reasoning Engine.
    ProposedAction {
        action_kind: String,
        action_args_json: String,
    },
    /// Action Router's verdict after a permission check.
    ActionApproved { action_kind: String },
    ActionDenied { action_kind: String, reason: String },
    /// Text ready to be spoken back to the user.
    SpeakText { text: String },
    /// Generic health/lifecycle signal used by the supervisor.
    Lifecycle(LifecycleEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LifecycleEvent {
    Started,
    Ready,
    ShuttingDown,
    /// A subsystem reports its own panic/failure before the supervisor
    /// restarts it. Distinct from an unexpected channel closure.
    Failed { reason: String },
}
