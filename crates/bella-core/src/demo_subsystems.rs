//! Minimal reference subsystems. These are not stand-ins for Voice/Vision/
//! Reasoning (those are real Phase 6/7/5 deliverables) — they exist so the
//! bus + supervisor + permission system can be exercised end-to-end right
//! now, with real async tasks, real messages, and a real permission check,
//! instead of asking you to trust an architecture diagram on faith.

use crate::subsystem_trait::Subsystem;
use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::info;
use bella_common::{Envelope, LifecycleEvent, MessageBus, Payload, SubsystemId, BellaResult};
use bella_memory::{MemoryEngine, NewEpisode};
use bella_permissions::{Capability, Grantee, PermissionSystem};

/// Stands in for the Voice subsystem: on receiving a `UserUtterance`, it
/// checks (via the real Permission System) that it's allowed to use the
/// microphone, then forwards a fabricated "assembled context" onward to
/// whatever destination the caller wired it to — demonstrating the actual
/// Voice -> Context Builder edge from the Phase 2 data-flow diagram.
pub struct DemoVoiceSubsystem {
    permissions: PermissionSystem,
    forward_to: SubsystemId,
}

impl DemoVoiceSubsystem {
    pub fn new(permissions: PermissionSystem, forward_to: SubsystemId) -> Self {
        Self {
            permissions,
            forward_to,
        }
    }
}

#[async_trait]
impl Subsystem for DemoVoiceSubsystem {
    fn id(&self) -> SubsystemId {
        SubsystemId::Voice
    }

    async fn run(&mut self, bus: MessageBus, mut rx: mpsc::Receiver<Envelope>) -> BellaResult<()> {
        while let Some(envelope) = rx.recv().await {
            match envelope.payload {
                Payload::UserUtterance { text } => {
                    let grantee = Grantee::Subsystem(SubsystemId::Voice);
                    match self.permissions.check(&grantee, &Capability::Microphone).await {
                        Ok(()) => {
                            info!(text = %text, "voice: heard utterance, permission granted, forwarding");
                            let out = Envelope::new(
                                SubsystemId::Voice,
                                self.forward_to,
                                envelope.correlation_id,
                                Payload::AssembledContext {
                                    context_json: format!("{{\"utterance\":{text:?}}}"),
                                },
                            );
                            bus.send(out).await?;
                        }
                        Err(e) => {
                            info!(error = %e, "voice: microphone permission denied, dropping utterance");
                        }
                    }
                }
                Payload::Lifecycle(LifecycleEvent::ShuttingDown) => {
                    info!("voice: shutdown requested, exiting run loop");
                    return Ok(());
                }
                other => {
                    info!(?other, "voice: ignoring unhandled payload variant");
                }
            }
        }
        Ok(())
    }
}

/// Stands in for the Context Builder: receives assembled context, records
/// it as an episode in the real Memory Engine, then recalls recent
/// episodes to demonstrate that memory persists across interactions —
/// this is the Phase 8 "episodic memory" deliverable actually wired into
/// the Phase 2 data flow, not a mock.
pub struct DemoContextSubsystem {
    memory: MemoryEngine,
}

impl DemoContextSubsystem {
    pub fn new(memory: MemoryEngine) -> Self {
        Self { memory }
    }
}

#[async_trait]
impl Subsystem for DemoContextSubsystem {
    fn id(&self) -> SubsystemId {
        SubsystemId::Context
    }

    async fn run(&mut self, _bus: MessageBus, mut rx: mpsc::Receiver<Envelope>) -> BellaResult<()> {
        while let Some(envelope) = rx.recv().await {
            match envelope.payload {
                Payload::AssembledContext { context_json } => {
                    let recorded = self
                        .memory
                        .record(NewEpisode {
                            correlation_id: envelope.correlation_id,
                            content: context_json.clone(),
                            kind: "assembled_context".to_string(),
                        })
                        .await;

                    match recorded {
                        Ok(episode) => {
                            info!(
                                correlation_id = %envelope.correlation_id,
                                episode_id = %episode.id,
                                context = %context_json,
                                "context: received and persisted assembled context"
                            );
                        }
                        Err(e) => {
                            info!(error = %e, "context: failed to persist episode to memory");
                        }
                    }

                    match self.memory.recent(3).await {
                        Ok(recent) => {
                            for ep in recent {
                                info!(
                                    episode_id = %ep.id,
                                    kind = %ep.kind,
                                    content = %ep.content,
                                    "context: recalled recent episode from memory"
                                );
                            }
                        }
                        Err(e) => {
                            info!(error = %e, "context: failed to recall recent episodes");
                        }
                    }
                }
                Payload::Lifecycle(LifecycleEvent::ShuttingDown) => {
                    info!("context: shutdown requested, exiting run loop");
                    return Ok(());
                }
                other => {
                    info!(?other, "context: ignoring unhandled payload variant");
                }
            }
        }
        Ok(())
    }
}
