//! Asanagi is a small, platform-agnostic framework for building chat bots.
//!
//! The core crate defines the shared message model, an [`Adapter`] trait for
//! platform integrations, and a [`Bot`] runtime that dispatches events to
//! asynchronous handlers. Platform-specific crates can implement [`Adapter`]
//! without depending on each other.
//!
//! ```no_run
//! use asanagi::{mock::MockAdapter, Bot, Handler, Context, Event, Result};
//! use async_trait::async_trait;
//!
//! struct Echo;
//!
//! #[async_trait]
//! impl Handler for Echo {
//!     async fn handle(&self, ctx: Context, event: Event) -> Result<()> {
//!         if let Event::Message(message) = event {
//!             ctx.reply(&message, &message.content).await?;
//!         }
//!         Ok(())
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> asanagi::Result<()> {
//!     let (adapter, _handle) = MockAdapter::new("mock");
//!     Bot::new(adapter).handler(Echo).run().await
//! }
//! ```

pub mod adapter;
pub mod bot;
pub mod capabilities;
pub mod context;
pub mod error;
pub mod handler;
pub mod mock;
pub mod model;

pub use adapter::{Adapter, EventSender};
pub use bot::Bot;
pub use capabilities::Capabilities;
pub use context::Context;
pub use error::{Error, Result};
pub use handler::Handler;
pub use model::{Attachment, Channel, ChannelKind, Event, Message, User};

/// Convenient imports for adapter and bot authors.
pub mod prelude {
    pub use crate::adapter::{Adapter, EventSender};
    pub use crate::bot::Bot;
    pub use crate::capabilities::Capabilities;
    pub use crate::context::Context;
    pub use crate::error::{Error, Result};
    pub use crate::handler::Handler;
    pub use crate::model::{Attachment, Channel, ChannelKind, Event, Message, User};
}
