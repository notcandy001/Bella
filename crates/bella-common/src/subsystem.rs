use serde::{Deserialize, Serialize};
use std::fmt;

/// Every subsystem in the daemon gets a stable identifier. This is the
/// addressing scheme for the message bus (Phase 2's "communication
/// protocol" decision) — subsystems address each other by SubsystemId,
/// never by holding a direct reference to one another's implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubsystemId {
    Core,
    Permissions,
    Memory,
    Context,
    Reasoning,
    ActionRouter,
    Voice,
    Vision,
    Device,
    PluginRuntime,
}

impl fmt::Display for SubsystemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            SubsystemId::Core => "core",
            SubsystemId::Permissions => "permissions",
            SubsystemId::Memory => "memory",
            SubsystemId::Context => "context",
            SubsystemId::Reasoning => "reasoning",
            SubsystemId::ActionRouter => "action_router",
            SubsystemId::Voice => "voice",
            SubsystemId::Vision => "vision",
            SubsystemId::Device => "device",
            SubsystemId::PluginRuntime => "plugin_runtime",
        };
        write!(f, "{s}")
    }
}
