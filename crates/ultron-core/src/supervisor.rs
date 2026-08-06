use crate::subsystem_trait::Subsystem;
use std::time::Duration;
use tokio::sync::watch;
use tracing::{error, info, warn};
use ultron_common::MessageBus;

/// How many times a subsystem may crash before the supervisor gives up on
/// it, rather than restart-looping forever and burning CPU on a subsystem
/// that will never recover (e.g. a missing device). This is the concrete
/// implementation of Phase 2's "recovery flow": a panic in one subsystem
/// must not take down the daemon, but it also must not be silently
/// swallowed forever.
const MAX_RESTARTS: u32 = 5;
const RESTART_BACKOFF: Duration = Duration::from_millis(500);

/// Owns the lifecycle of every subsystem task. The supervisor is the only
/// piece of code in the daemon allowed to spawn or restart a subsystem —
/// no subsystem restarts itself, which keeps restart policy centralized
/// and auditable in one place instead of scattered across modules.
pub struct Supervisor {
    bus: MessageBus,
    shutdown_tx: watch::Sender<bool>,
    shutdown_rx: watch::Receiver<bool>,
}

impl Supervisor {
    pub fn new(bus: MessageBus) -> Self {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        Self {
            bus,
            shutdown_tx,
            shutdown_rx,
        }
    }

    pub fn bus(&self) -> &MessageBus {
        &self.bus
    }

    /// Signal every supervised task to stop. Tasks observe this via their
    /// own watch::Receiver clone and exit their run loop cooperatively —
    /// we don't hard-abort tasks, so in-flight work (e.g. a memory write)
    /// isn't torn down mid-operation.
    pub fn request_shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
    }

    /// Spawn a subsystem under supervision. `factory` produces a fresh
    /// instance on every (re)start — required because a subsystem that
    /// panicked may be holding corrupted internal state, so we never
    /// resume the same instance, only ever a clean one.
    pub fn supervise<F, S>(&self, factory: F)
    where
        F: Fn() -> S + Send + 'static,
        S: Subsystem,
    {
        let bus = self.bus.clone();
        let mut shutdown_rx = self.shutdown_rx.clone();

        tokio::spawn(async move {
            let mut restarts: u32 = 0;
            loop {
                if *shutdown_rx.borrow() {
                    return;
                }

                let mut instance = factory();
                let id = instance.id();
                let rx = bus.register(id).await;
                info!(subsystem = %id, attempt = restarts, "starting subsystem");

                let task_bus = bus.clone();
                let run_result: Result<ultron_common::UltronResult<()>, tokio::task::JoinError> =
                    tokio::spawn(async move { instance.run(task_bus, rx).await }).await;

                bus.deregister(id).await;

                match run_result {
                    Ok(Ok(())) => {
                        info!(subsystem = %id, "subsystem exited cleanly, not restarting");
                        return;
                    }
                    Ok(Err(e)) => {
                        warn!(subsystem = %id, error = %e, "subsystem returned an error");
                    }
                    Err(join_err) => {
                        error!(subsystem = %id, error = %join_err, "subsystem task panicked");
                    }
                }

                if *shutdown_rx.borrow() {
                    return;
                }

                restarts += 1;
                if restarts > MAX_RESTARTS {
                    error!(
                        subsystem = %id,
                        max_restarts = MAX_RESTARTS,
                        "subsystem exceeded max restarts, giving up"
                    );
                    return;
                }

                tokio::select! {
                    _ = tokio::time::sleep(RESTART_BACKOFF) => {}
                    _ = shutdown_rx.changed() => {
                        if *shutdown_rx.borrow() {
                            return;
                        }
                    }
                }
            }
        });
    }
}
