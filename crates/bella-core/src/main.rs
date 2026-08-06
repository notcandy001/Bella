use std::time::Duration;
use tracing::info;
use bella_common::{Envelope, MessageBus, Payload, SubsystemId};
use bella_core::demo_subsystems::{DemoContextSubsystem, DemoVoiceSubsystem};
use bella_core::Supervisor;
use bella_memory::MemoryEngine;
use bella_permissions::{Capability, Grantee, PermissionSystem};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("bella daemon starting (Phase 5 milestone: bus + permissions + supervisor + memory)");

    let bus = MessageBus::new();
    let permissions = PermissionSystem::new();
    let memory = MemoryEngine::open("bella_memory.db")
        .expect("failed to open memory database at ./bella_memory.db");

    // Phase 1 requirement: explicit grants, no ambient authority. The
    // daemon itself decides, at startup, what Voice is allowed to touch —
    // in a full build this would come from user-facing settings /
    // first-run consent, not be hardcoded like this demo does.
    permissions
        .grant(
            Grantee::Subsystem(SubsystemId::Voice),
            Capability::Microphone,
            None,
        )
        .await;

    let supervisor = Supervisor::new(bus.clone());

    {
        let permissions = permissions.clone();
        supervisor.supervise(move || DemoVoiceSubsystem::new(permissions.clone(), SubsystemId::Context));
    }
    {
        let memory = memory.clone();
        supervisor.supervise(move || DemoContextSubsystem::new(memory.clone()));
    }

    // Give the subsystems a moment to register on the bus before we start
    // addressing them by SubsystemId.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Drive two interactions through the real pipeline so the log shows
    // memory actually accumulating: Voice -> permission check -> Context
    // -> Memory Engine write -> Memory Engine recall of recent episodes,
    // which on the second utterance will include the first.
    for text in [
        "what's on my calendar today",
        "remind me to buy groceries later",
    ] {
        let utterance = Envelope::new_root(
            SubsystemId::Core,
            SubsystemId::Voice,
            Payload::UserUtterance {
                text: text.to_string(),
            },
        );
        if let Err(e) = bus.send(utterance).await {
            tracing::warn!(error = %e, "failed to deliver demo utterance");
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    for entry in permissions.audit_log().await {
        info!(
            grantee = ?entry.grantee,
            capability = %entry.capability,
            decision = ?entry.decision,
            "audit"
        );
    }

    let total = memory.count().await.unwrap_or(0);
    info!(episodes_stored = total, "memory engine status");

    info!("demo interaction complete, waiting for Ctrl+C to shut down");
    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for shutdown signal");

    info!("shutdown requested");
    supervisor.request_shutdown();
    tokio::time::sleep(Duration::from_millis(100)).await;
    info!("bella daemon stopped");
}
