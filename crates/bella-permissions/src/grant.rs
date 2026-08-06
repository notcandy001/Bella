use crate::capability::Capability;
use bella_common::SubsystemId;
use std::time::{SystemTime, UNIX_EPOCH};

/// Who a capability was granted to. Subsystems and plugins are both
/// grantable, but a plugin grant is scoped by plugin id so revoking one
/// plugin's access can never accidentally revoke another's.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Grantee {
    Subsystem(SubsystemId),
    Plugin(String),
}

/// A live permission grant. Grants are explicit and revocable, per Phase 1
/// — there is deliberately no "grant forever, no record" path.
#[derive(Debug, Clone)]
pub struct Grant {
    pub grantee: Grantee,
    pub capability: Capability,
    pub granted_at_unix: u64,
    /// None = does not expire until explicitly revoked. Time-bounded
    /// grants (e.g. "microphone for the next 10 minutes") are supported
    /// by setting this.
    pub expires_at_unix: Option<u64>,
}

impl Grant {
    pub fn new(grantee: Grantee, capability: Capability, ttl_secs: Option<u64>) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_secs();
        Self {
            grantee,
            capability,
            granted_at_unix: now,
            expires_at_unix: ttl_secs.map(|ttl| now + ttl),
        }
    }

    pub fn is_expired(&self, now_unix: u64) -> bool {
        match self.expires_at_unix {
            Some(exp) => now_unix >= exp,
            None => false,
        }
    }
}

/// One entry in the append-only audit trail (Phase 1 security requirement:
/// "all device-control actions logged to an append-only audit trail").
/// The Permission System owns this log because every privileged action
/// must pass through a permission check first, making it the single
/// natural choke point to record from.
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub timestamp_unix: u64,
    pub grantee: Grantee,
    pub capability: Capability,
    pub decision: AuditDecision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditDecision {
    Allowed,
    DeniedNoGrant,
    DeniedExpired,
}
