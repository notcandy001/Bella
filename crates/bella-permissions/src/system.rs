use crate::capability::Capability;
use crate::grant::{AuditDecision, AuditEntry, Grant, Grantee};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use bella_common::{BellaError, BellaResult};

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_secs()
}

/// The Permission System: the one subsystem in the Phase 2 dependency
/// graph with no dependencies on any other subsystem. Every privileged
/// action anywhere in the daemon (device control, plugin calls, shell
/// exec) must call `check` here before executing, and every grant/deny
/// decision is recorded in the audit trail.
#[derive(Clone)]
pub struct PermissionSystem {
    grants: Arc<RwLock<HashMap<Grantee, Vec<Grant>>>>,
    audit_log: Arc<RwLock<Vec<AuditEntry>>>,
}

impl PermissionSystem {
    pub fn new() -> Self {
        Self {
            grants: Arc::new(RwLock::new(HashMap::new())),
            audit_log: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Explicitly grant a capability. There is no implicit/default grant
    /// path anywhere in this crate — every capability a subsystem or
    /// plugin ends up with came through this function.
    pub async fn grant(&self, grantee: Grantee, capability: Capability, ttl_secs: Option<u64>) {
        let grant = Grant::new(grantee.clone(), capability, ttl_secs);
        let mut grants = self.grants.write().await;
        grants.entry(grantee).or_default().push(grant);
    }

    /// Revoke every grant of this exact capability for this grantee.
    /// Returns the number of grants removed, mainly useful for tests/logging.
    pub async fn revoke(&self, grantee: &Grantee, capability: &Capability) -> usize {
        let mut grants = self.grants.write().await;
        if let Some(list) = grants.get_mut(grantee) {
            let before = list.len();
            list.retain(|g| &g.capability != capability);
            before - list.len()
        } else {
            0
        }
    }

    /// Revoke all grants for a grantee at once — used when a plugin is
    /// unloaded, or a subsystem is torn down.
    pub async fn revoke_all(&self, grantee: &Grantee) {
        self.grants.write().await.remove(grantee);
    }

    /// The enforcement point. Returns Ok(()) only if the grantee holds a
    /// live (non-expired) grant that permits the requested capability.
    /// Every call, allowed or denied, is recorded to the audit trail —
    /// this function is the single choke point the audit log design in
    /// Phase 1 depends on.
    pub async fn check(&self, grantee: &Grantee, requested: &Capability) -> BellaResult<()> {
        let now = now_unix();
        let grants = self.grants.read().await;

        let decision = match grants.get(grantee) {
            None => AuditDecision::DeniedNoGrant,
            Some(list) => {
                let mut saw_expired_match = false;
                let mut allowed = false;
                for g in list {
                    if g.capability.permits(requested) {
                        if g.is_expired(now) {
                            saw_expired_match = true;
                        } else {
                            allowed = true;
                            break;
                        }
                    }
                }
                if allowed {
                    AuditDecision::Allowed
                } else if saw_expired_match {
                    AuditDecision::DeniedExpired
                } else {
                    AuditDecision::DeniedNoGrant
                }
            }
        };
        drop(grants);

        self.audit_log.write().await.push(AuditEntry {
            timestamp_unix: now,
            grantee: grantee.clone(),
            capability: requested.clone(),
            decision: decision.clone(),
        });

        match decision {
            AuditDecision::Allowed => Ok(()),
            _ => Err(BellaError::PermissionDenied(requested.to_string())),
        }
    }

    /// Read-only snapshot of the audit trail, e.g. for the future
    /// "Memory Explorer" / developer-mode UI in Phase 16.
    pub async fn audit_log(&self) -> Vec<AuditEntry> {
        self.audit_log.read().await.clone()
    }
}

impl Default for PermissionSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bella_common::SubsystemId;

    #[tokio::test]
    async fn ungranted_capability_is_denied_and_audited() {
        let perms = PermissionSystem::new();
        let grantee = Grantee::Subsystem(SubsystemId::Voice);
        let result = perms.check(&grantee, &Capability::Microphone).await;
        assert!(matches!(result, Err(BellaError::PermissionDenied(_))));

        let log = perms.audit_log().await;
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].decision, AuditDecision::DeniedNoGrant);
    }

    #[tokio::test]
    async fn granted_capability_is_allowed() {
        let perms = PermissionSystem::new();
        let grantee = Grantee::Subsystem(SubsystemId::Voice);
        perms.grant(grantee.clone(), Capability::Microphone, None).await;

        let result = perms.check(&grantee, &Capability::Microphone).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn revoke_removes_access() {
        let perms = PermissionSystem::new();
        let grantee = Grantee::Subsystem(SubsystemId::Device);
        perms.grant(grantee.clone(), Capability::ShellExec, None).await;
        assert!(perms.check(&grantee, &Capability::ShellExec).await.is_ok());

        let removed = perms.revoke(&grantee, &Capability::ShellExec).await;
        assert_eq!(removed, 1);
        assert!(perms.check(&grantee, &Capability::ShellExec).await.is_err());
    }

    #[tokio::test]
    async fn expired_grant_is_denied() {
        let perms = PermissionSystem::new();
        let grantee = Grantee::Plugin("weather-plugin".into());
        // TTL of 0 seconds: expires immediately.
        perms
            .grant(grantee.clone(), Capability::Network, Some(0))
            .await;

        let result = perms.check(&grantee, &Capability::Network).await;
        assert!(result.is_err());
        let log = perms.audit_log().await;
        assert_eq!(log.last().unwrap().decision, AuditDecision::DeniedExpired);
    }

    #[tokio::test]
    async fn filesystem_prefix_grant_covers_subpaths_only() {
        let perms = PermissionSystem::new();
        let grantee = Grantee::Plugin("notes-plugin".into());
        perms
            .grant(
                grantee.clone(),
                Capability::FilesystemWrite {
                    path_prefix: "/home/notcandy/notes".into(),
                },
                None,
            )
            .await;

        let inside = Capability::FilesystemWrite {
            path_prefix: "/home/notcandy/notes/today.md".into(),
        };
        let outside = Capability::FilesystemWrite {
            path_prefix: "/home/notcandy/.ssh/id_rsa".into(),
        };

        assert!(perms.check(&grantee, &inside).await.is_ok());
        assert!(perms.check(&grantee, &outside).await.is_err());
    }

    #[tokio::test]
    async fn plugin_grants_are_isolated_by_plugin_id() {
        let perms = PermissionSystem::new();
        let plugin_a = Grantee::Plugin("plugin-a".into());
        let plugin_b = Grantee::Plugin("plugin-b".into());
        perms
            .grant(plugin_a.clone(), Capability::Network, None)
            .await;

        assert!(perms.check(&plugin_a, &Capability::Network).await.is_ok());
        assert!(perms.check(&plugin_b, &Capability::Network).await.is_err());
    }

    #[tokio::test]
    async fn revoke_all_clears_every_grant_for_grantee() {
        let perms = PermissionSystem::new();
        let grantee = Grantee::Plugin("temp-plugin".into());
        perms.grant(grantee.clone(), Capability::Network, None).await;
        perms.grant(grantee.clone(), Capability::Clipboard, None).await;

        perms.revoke_all(&grantee).await;

        assert!(perms.check(&grantee, &Capability::Network).await.is_err());
        assert!(perms.check(&grantee, &Capability::Clipboard).await.is_err());
    }
}
