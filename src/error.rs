use std::fmt;

/// Errors returned by adapters, handlers, and the bot runtime.
#[derive(Debug)]
pub enum Error {
    /// An adapter failed to start, connect, or send a message.
    Adapter(String),
    /// A handler returned an error.
    Handler(String),
    /// An internal runtime task failed or exited unexpectedly.
    Runtime(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Adapter(message) => write!(f, "adapter error: {message}"),
            Self::Handler(message) => write!(f, "handler error: {message}"),
            Self::Runtime(message) => write!(f, "runtime error: {message}"),
        }
    }
}

impl std::error::Error for Error {}

/// Convenience result type used throughout the framework.
pub type Result<T, E = Error> = std::result::Result<T, E>;
