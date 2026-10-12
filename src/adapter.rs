use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::{
    Result,
    capabilities::Capabilities,
    model::{Event, Message, User},
};

/// Sender given to adapters for emitting normalized events.
pub type EventSender = mpsc::Sender<Event>;

/// A connection to a chat platform.
///
/// Implementations translate platform-specific API types into [`Event`] values
/// and translate outgoing [`Message`] values back into platform calls. Adapter
/// methods take `&self` so the same adapter can be shared by the runtime and
/// handler contexts; implementations should use interior mutability for
/// network clients or session state.
#[async_trait]
pub trait Adapter: Send + Sync {
    /// Stable adapter name, such as `"discord"` or `"qq"`.
    fn name(&self) -> &str;

    /// The bot account represented by this adapter.
    fn self_user(&self) -> User;

    /// Features supported by this adapter.
    ///
    /// Adapters should return the set of operations they genuinely implement,
    /// not merely those available on the underlying platform.
    fn capabilities(&self) -> Capabilities;

    /// Connect and forward events to `events` until the connection ends.
    ///
    /// Returning normally signals that no more events will be produced.
    async fn start(&self, events: EventSender) -> Result<()>;

    /// Send text to a channel and return the normalized outgoing message.
    async fn send_message(
        &self,
        channel_id: &str,
        content: &str,
        reply_to: Option<&str>,
    ) -> Result<Message>;
}
