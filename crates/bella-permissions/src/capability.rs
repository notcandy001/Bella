use serde::{Deserialize, Serialize};
use std::fmt;

/// The full set of privileged capabilities any subsystem or plugin can
/// request. This enum is the enforcement point for Phase 1's "explicit,
/// revocable permission grants per capability... no ambient authority"
/// requirement — a subsystem cannot touch the filesystem, mic, or shell
/// unless it holds a live Grant for the matching variant, checked at the
/// point of use (in the Action Router), not just at startup.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    FilesystemRead { path_prefix: String },
    FilesystemWrite { path_prefix: String },
    Microphone,
    Camera,
    ScreenCapture,
    ShellExec,
    InputSimulation,
    Clipboard,
    Network,
    Notification,
    /// Plugins get a namespaced capability so one plugin's grant can never
    /// be mistaken for another's, even if both request "the same" thing.
    Plugin { plugin_id: String, capability: String },
}

impl Capability {
    /// Does a granted capability (`self`) satisfy a requested one
    /// (`requested`)? Filesystem capabilities use prefix matching (a grant
    /// for "/home/user/documents" covers a request for
    /// "/home/user/documents/report.txt") — everything else requires an
    /// exact match. This asymmetry (granted vs requested) is deliberate:
    /// it's what lets us grant scoped, narrow filesystem access instead of
    /// all-or-nothing.
    pub fn permits(&self, requested: &Capability) -> bool {
        match (self, requested) {
            (
                Capability::FilesystemRead { path_prefix: granted },
                Capability::FilesystemRead { path_prefix: req },
            ) => req.starts_with(granted.as_str()),
            (
                Capability::FilesystemWrite { path_prefix: granted },
                Capability::FilesystemWrite { path_prefix: req },
            ) => req.starts_with(granted.as_str()),
            _ => self == requested,
        }
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Capability::FilesystemRead { path_prefix } => {
                write!(f, "filesystem:read:{path_prefix}")
            }
            Capability::FilesystemWrite { path_prefix } => {
                write!(f, "filesystem:write:{path_prefix}")
            }
            Capability::Microphone => write!(f, "microphone"),
            Capability::Camera => write!(f, "camera"),
            Capability::ScreenCapture => write!(f, "screen_capture"),
            Capability::ShellExec => write!(f, "shell_exec"),
            Capability::InputSimulation => write!(f, "input_simulation"),
            Capability::Clipboard => write!(f, "clipboard"),
            Capability::Network => write!(f, "network"),
            Capability::Notification => write!(f, "notification"),
            Capability::Plugin {
                plugin_id,
                capability,
            } => write!(f, "plugin:{plugin_id}:{capability}"),
        }
    }
}
