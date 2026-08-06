use thiserror::Error;

/// Crate-wide error type. Every subsystem's public interface returns this
/// (or wraps it) so the Action Router and supervisor can reason about
/// failures uniformly instead of every module inventing its own error type.
#[derive(Debug, Error)]
pub enum BellaError {
    #[error("permission denied: capability '{0}' was not granted")]
    PermissionDenied(String),

    #[error("subsystem '{0}' is not registered on the message bus")]
    SubsystemNotFound(String),

    #[error("message bus channel closed for subsystem '{0}'")]
    ChannelClosed(String),

    #[error("message bus is full for subsystem '{0}' (backpressure)")]
    ChannelFull(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("subsystem-internal error: {0}")]
    Internal(String),
}

pub type BellaResult<T> = Result<T, BellaError>;
