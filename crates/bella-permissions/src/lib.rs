//! bella-permissions: the Permission System. Per the Phase 2 dependency
//! graph, this is the one subsystem every other subsystem depends on, and
//! it depends on none of them — only on bella-common for shared types.

pub mod capability;
pub mod grant;
pub mod system;

pub use capability::Capability;
pub use grant::{AuditDecision, AuditEntry, Grant, Grantee};
pub use system::PermissionSystem;
