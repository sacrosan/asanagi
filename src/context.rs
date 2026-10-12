use std::sync::Arc;

use crate::{
    Result,
    adapter::Adapter,
    model::{Message, User},
};

/// Per-event context providing access to the source adapter.
#[derive(Clone)]
pub struct Context {
    adapter: Arc<dyn Adapter>,
}

impl Context {
    pub(crate) fn new(adapter: Arc<dyn Adapter>) -> Self {
        Self { adapter }
    }

    pub fn adapter_name(&self) -> &str {
        self.adapter.name()
    }

    pub fn self_user(&self) -> User {
        self.adapter.self_user()
    }

    /// Send a message to a channel.
    pub async fn send_message(&self, channel_id: &str, content: &str) -> Result<Message> {
        self.adapter.send_message(channel_id, content, None).await
    }

    /// Send a message to the channel that `message` came from.
    ///
    /// This is a plain channel reply. Where supported by the source platform,
    /// `reply_to` is used to reference the original message.
    pub async fn reply(&self, message: &Message, content: &str) -> Result<Message> {
        self.adapter
            .send_message(&message.channel_id, content, message.reply_to.as_deref())
            .await
    }
}
