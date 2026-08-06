use std::time::Duration;
use tracing::info;
use ultron_common::{Envelope, MessageBus, Payload, SubsystemId};
use ultron_core::demo_subsystems::{DemoContextSubsystem, DemoVoiceSubsystem};
use ultron_core::Supervisor;
use ultron_permissions::{Capability, Grantee, PermissionSystem};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("ultron daemon starting (Phase 5 milestone: bus + permissions + supervisor)");

    let bus = MessageBus::new();
    let permissions = PermissionSystem::new();

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
    supervisor.supervise(|| DemoContextSubsystem);

    // Give the subsystems a moment to register on the bus before we start
    // addressing them by SubsystemId.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Drive one full interaction through the real pipeline: Voice receives
    // an utterance, checks a real permission grant, forwards to Context.
    // This is the Phase 2 "voice flow" edge, actually executing.
    let demo_utterance = Envelope::new_root(
        SubsystemId::Core,
        SubsystemId::Voice,
        Payload::UserUtterance {
            text: "what's on my calendar today".to_string(),
        },
    );
    if let Err(e) = bus.send(demo_utterance).await {
        tracing::warn!(error = %e, "failed to deliver demo utterance");
    }

    // Let the message propagate and get logged by the Context subsystem.
    tokio::time::sleep(Duration::from_millis(100)).await;

    for entry in permissions.audit_log().await {
        info!(
            grantee = ?entry.grantee,
            capability = %entry.capability,
            decision = ?entry.decision,
            "audit"
        );
    }

    info!("demo interaction complete, waiting for Ctrl+C to shut down");
    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for shutdown signal");

    info!("shutdown requested");
    supervisor.request_shutdown();
    tokio::time::sleep(Duration::from_millis(100)).await;
    info!("ultron daemon stopped");
}
