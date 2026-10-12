//! Discord adapter for Asanagi.
//!
//! ```no_run
//! use asanagi::{Bot, Handler};
//! use asanagi_discord::{DiscordAdapter, DiscordConfig};
//!
//! struct Echo;
//!
//! #[async_trait::async_trait]
//! impl Handler for Echo {
//!     async fn handle(
//!         &self,
//!         ctx: asanagi::Context,
//!         event: asanagi::Event,
//!     ) -> asanagi::Result<()> {
//!         if let asanagi::Event::Message(message) = event {
//!             ctx.reply(&message, &message.content).await?;
//!         }
//!         Ok(())
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> asanagi::Result<()> {
//!     let adapter = DiscordAdapter::new(DiscordConfig::from_env()?);
//!     Bot::new(adapter).handler(Echo).run().await
//! }
//! ```

use std::sync::OnceLock;

use async_trait::async_trait;
use serenity::{
    all::{ChannelId, CreateMessage, GatewayIntents, Message as DiscordMessage, MessageId},
    async_trait as serenity_trait,
    client::{Client, Context as SerenityContext, EventHandler},
    http::Http,
    model::prelude::Ready,
};
use tokio::sync::RwLock;

use asanagi::{
    Error, Result,
    adapter::{Adapter, EventSender},
    capabilities::Capabilities,
    model::{Attachment, Event, Message, User},
};

/// Discord adapter configuration.
#[derive(Debug, Clone)]
pub struct DiscordConfig {
    pub token: String,
    pub intents: GatewayIntents,
}

impl DiscordConfig {
    /// Load `DISCORD_TOKEN` from the environment.
    pub fn from_env() -> Result<Self> {
        let token = std::env::var("DISCORD_TOKEN")
            .map_err(|_| Error::Adapter("DISCORD_TOKEN is not set".into()))?;

        Ok(Self {
            token,
            intents: GatewayIntents::non_privileged(),
        })
    }

    /// Enable additional privileged intents.
    pub fn with_intents(mut self, intents: GatewayIntents) -> Self {
        self.intents |= intents;
        self
    }
}

struct DiscordState {
    events: EventSender,
    self_user: RwLock<Option<User>>,
}

#[serenity_trait]
impl EventHandler for DiscordState {
    async fn ready(&self, _: SerenityContext, ready: Ready) {
        let user = User {
            id: ready.user.id.to_string(),
            username: Some(ready.user.name.clone()),
            display_name: ready.user.global_name.clone(),
            is_bot: ready.user.bot,
        };

        let _ = self.events.send(Event::Connected(user.clone())).await;
        *self.self_user.write().await = Some(user);
    }

    async fn message(&self, _: SerenityContext, message: DiscordMessage) {
        let event = Event::Message(normalize_message(&message));
        let _ = self.events.send(event).await;
    }
}

/// Adapter for the Discord gateway and HTTP APIs.
pub struct DiscordAdapter {
    config: DiscordConfig,
    http: OnceLock<ArcHttp>,
    self_user: OnceLock<User>,
}

#[derive(Clone)]
struct ArcHttp(std::sync::Arc<Http>);

impl DiscordAdapter {
    pub fn new(config: DiscordConfig) -> Self {
        Self {
            config,
            http: OnceLock::new(),
            self_user: OnceLock::new(),
        }
    }

    fn http(&self) -> Result<std::sync::Arc<Http>> {
        self.http
            .get()
            .map(|http| http.0.clone())
            .ok_or_else(|| Error::Adapter("Discord adapter has not connected yet".into()))
    }
}

#[async_trait]
impl Adapter for DiscordAdapter {
    fn name(&self) -> &str {
        "discord"
    }

    fn self_user(&self) -> User {
        self.self_user
            .get()
            .cloned()
            .unwrap_or_else(|| User::new("discord-bot").with_is_bot(true))
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::from_capabilities(&[
            Capabilities::SEND_MESSAGES,
            Capabilities::EDIT_MESSAGES,
            Capabilities::DELETE_MESSAGES,
            Capabilities::ADD_REACTIONS,
            Capabilities::REMOVE_REACTIONS,
            Capabilities::BROADCAST_TYPING,
        ])
    }

    async fn start(&self, events: EventSender) -> Result<()> {
        let state = DiscordState {
            events,
            self_user: RwLock::new(None),
        };
        let http = std::sync::Arc::new(Http::new(&self.config.token));
        let intents = self.config.intents;

        let mut client = Client::builder(&self.config.token, intents)
            .event_handler(state)
            .await
            .map_err(|err| Error::Adapter(format!("failed to create Discord client: {err}")))?;

        self.http
            .set(ArcHttp(http))
            .map_err(|_| Error::Runtime("Discord HTTP client was already initialized".into()))?;

        client
            .start()
            .await
            .map_err(|err| Error::Adapter(format!("Discord gateway disconnected: {err}")))
    }

    async fn send_message(
        &self,
        channel_id: &str,
        content: &str,
        reply_to: Option<&str>,
    ) -> Result<Message> {
        let http = self.http()?;
        let channel = parse_snowflake(channel_id)?;
        let mut builder = CreateMessage::new().content(content);

        if let Some(reply_to) = reply_to {
            let reply_id = parse_snowflake(reply_to)?;
            builder =
                builder.reference_message((ChannelId::new(channel), MessageId::new(reply_id)));
        }

        let sent = ChannelId::new(channel)
            .send_message(&http, builder)
            .await
            .map_err(|err| Error::Adapter(format!("failed to send Discord message: {err}")))?;

        Ok(normalize_message(&sent))
    }
}

fn parse_snowflake(value: &str) -> Result<u64> {
    value
        .parse()
        .map_err(|_| Error::Adapter(format!("invalid Discord snowflake: {value}")))
}

fn normalize_message(message: &DiscordMessage) -> Message {
    let author = User {
        id: message.author.id.to_string(),
        username: Some(message.author.name.clone()),
        display_name: message
            .author
            .global_name
            .clone()
            .or_else(|| message.member.as_ref().map(|m| m.nick.clone()).flatten()),
        is_bot: message.author.bot,
    };

    let attachments = message
        .attachments
        .iter()
        .map(|attachment| Attachment {
            id: attachment.id.to_string(),
            filename: attachment.filename.clone(),
            url: Some(attachment.url.clone()),
            content_type: attachment.content_type.clone(),
        })
        .collect();

    let mut normalized = Message::new(
        message.id.to_string(),
        message.channel_id.to_string(),
        author,
        message.content.clone(),
    )
    .with_attachments(attachments);

    if let Some(reference) = &message.message_reference {
        if let Some(id) = reference.message_id {
            normalized = normalized.with_reply_to(id.to_string());
        }
    }

    normalized
}

trait MessageExt {
    fn with_attachments(self, attachments: Vec<Attachment>) -> Self;
    fn with_reply_to(self, reply_to: impl Into<String>) -> Self;
}

impl MessageExt for Message {
    fn with_attachments(mut self, attachments: Vec<Attachment>) -> Self {
        self.attachments = attachments;
        self
    }

    fn with_reply_to(mut self, reply_to: impl Into<String>) -> Self {
        self.reply_to = Some(reply_to.into());
        self
    }
}
