use std::time::SystemTime;

/// A user or bot account on a messaging platform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: String,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub is_bot: bool,
}

impl User {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            username: None,
            display_name: None,
            is_bot: false,
        }
    }

    pub fn with_is_bot(mut self, is_bot: bool) -> Self {
        self.is_bot = is_bot;
        self
    }

    /// Best available human-readable name.
    pub fn label(&self) -> &str {
        self.display_name
            .as_deref()
            .or(self.username.as_deref())
            .unwrap_or(&self.id)
    }
}

/// The kind of conversation a message was sent in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelKind {
    Direct,
    Group,
    Server,
    Unknown,
}

/// A conversation, such as a Discord channel, QQ group, or Telegram chat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Channel {
    pub id: String,
    pub name: Option<String>,
    pub kind: ChannelKind,
}

impl Channel {
    pub fn new(id: impl Into<String>, kind: ChannelKind) -> Self {
        Self {
            id: id.into(),
            name: None,
            kind,
        }
    }
}

/// A file, image, or other non-text attachment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attachment {
    pub id: String,
    pub filename: String,
    pub url: Option<String>,
    pub content_type: Option<String>,
}

/// A normalized chat message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub id: String,
    pub channel_id: String,
    pub author: User,
    pub content: String,
    pub attachments: Vec<Attachment>,
    pub timestamp: Option<SystemTime>,
    /// ID of the message this message replies to, if any.
    pub reply_to: Option<String>,
}

impl Message {
    pub fn new(
        id: impl Into<String>,
        channel_id: impl Into<String>,
        author: User,
        content: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            channel_id: channel_id.into(),
            author,
            content: content.into(),
            attachments: Vec::new(),
            timestamp: None,
            reply_to: None,
        }
    }
}

/// Events emitted by an adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// The adapter authenticated and is ready to receive traffic.
    Connected(User),
    /// A new message was received.
    Message(Message),
}

impl Event {
    /// Returns the message for [`Event::Message`].
    pub fn as_message(&self) -> Option<&Message> {
        match self {
            Self::Message(message) => Some(message),
            _ => None,
        }
    }
}
