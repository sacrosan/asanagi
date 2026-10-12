use std::fmt;

/// Errors returned by adapters, handlers, and the bot runtime.
#[derive(Debug)]
pub enum Error {
    /// An adapter failed to start, connect, or send a message.
    Adapter(String),
    /// The operation is not supported by the source adapter.
    Unsupported {
        operation: &'static str,
        adapter: String,
    },
    /// A handler returned an error.
    Handler(String),
    /// An internal runtime task failed or exited unexpectedly.
    Runtime(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Adapter(message) => write!(f, "adapter error: {message}"),
            Self::Unsupported { operation, adapter } => {
                write!(f, "{adapter} does not support {operation}")
            }
            Self::Handler(message) => write!(f, "handler error: {message}"),
            Self::Runtime(message) => write!(f, "runtime error: {message}"),
        }
    }
}

impl std::error::Error for Error {}

/// Convenience result type used throughout the framework.
pub type Result<T, E = Error> = std::result::Result<T, E>;
