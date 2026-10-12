//! OneBot v11 adapter for Asanagi.
//!
//! This adapter connects as a WebSocket client to a OneBot implementation
//! such as NapCat, Lagrange.OneBot, or LLOneBot.
//!
//! ```no_run
//! use asanagi::{Bot, Handler};
//! use asanagi_onebot::{OneBotAdapter, OneBotConfig};
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
//!     let adapter = OneBotAdapter::new(OneBotConfig::from_env()?);
//!     Bot::new(adapter).handler(Echo).run().await
//! }
//! ```

use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicI64, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use tokio::sync::{Mutex, RwLock, mpsc, oneshot};
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};

use asanagi::{
    Error, Result,
    adapter::{Adapter, EventSender},
    capabilities::Capabilities,
    model::{Attachment, Event, Message, User},
};

/// OneBot adapter configuration.
#[derive(Debug, Clone)]
pub struct OneBotConfig {
    pub websocket_url: String,
    pub access_token: Option<String>,
    pub request_timeout: Duration,
}

impl OneBotConfig {
    /// Load connection settings from the environment.
    ///
    /// - `ONEBOT_WS_URL`: required WebSocket endpoint, e.g. `ws://localhost:3001`.
    /// - `ONEBOT_ACCESS_TOKEN`: optional bearer token.
    pub fn from_env() -> Result<Self> {
        let websocket_url = std::env::var("ONEBOT_WS_URL")
            .map_err(|_| Error::Adapter("ONEBOT_WS_URL is not set".into()))?;

        Ok(Self {
            websocket_url,
            access_token: std::env::var("ONEBOT_ACCESS_TOKEN").ok(),
            request_timeout: Duration::from_secs(15),
        })
    }
}

#[derive(Default)]
struct PendingRequests {
    counter: AtomicI64,
    requests: Mutex<HashMap<String, oneshot::Sender<Result<serde_json::Value>>>>,
}

/// Adapter for a OneBot v11 implementation.
pub struct OneBotAdapter {
    config: OneBotConfig,
    self_user: RwLock<Option<User>>,
    writer: Arc<Mutex<Option<Writer>>>,
    pending: Arc<PendingRequests>,
}

type Writer = futures::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    WsMessage,
>;

impl OneBotAdapter {
    pub fn new(config: OneBotConfig) -> Self {
        Self {
            config,
            self_user: RwLock::new(None),
            writer: Arc::new(Mutex::new(None)),
            pending: Arc::new(PendingRequests::default()),
        }
    }

    async fn send_request(
        &self,
        action: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let echo = format!(
            "asanagi-{}",
            self.pending.counter.fetch_add(1, Ordering::Relaxed)
        );
        let (tx, rx) = oneshot::channel();
        self.pending.requests.lock().await.insert(echo.clone(), tx);

        let payload = serde_json::json!({
            "action": action,
            "params": params,
            "echo": echo,
        });

        let writer_slot = self.writer.clone();
        let mut writer = writer_slot.lock().await;
        let writer = writer
            .as_mut()
            .ok_or_else(|| Error::Adapter("OneBot adapter has not connected yet".into()))?;
        writer
            .send(WsMessage::Text(payload.to_string().into()))
            .await
            .map_err(|err| Error::Adapter(format!("failed to send OneBot request: {err}")))?;

        match tokio::time::timeout(self.config.request_timeout, rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(Error::Adapter("OneBot request channel closed".into())),
            Err(_) => Err(Error::Adapter("OneBot request timed out".into())),
        }
    }
}

#[async_trait]
impl Adapter for OneBotAdapter {
    fn name(&self) -> &str {
        "onebot"
    }

    fn self_user(&self) -> User {
        tokio::task::block_in_place(|| futures::executor::block_on(self.self_user.read()))
            .clone()
            .unwrap_or_else(|| User::new("onebot-self").with_is_bot(true))
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities::from_capabilities(&[
            Capabilities::SEND_MESSAGES,
            Capabilities::EDIT_MESSAGES,
            Capabilities::DELETE_MESSAGES,
            Capabilities::ADD_REACTIONS,
            Capabilities::REMOVE_REACTIONS,
        ])
    }

    async fn start(&self, events: EventSender) -> Result<()> {
        let (stream, _) = connect_async(&self.config.websocket_url)
            .await
            .map_err(|err| Error::Adapter(format!("failed to connect to OneBot: {err}")))?;

        let (writer, mut reader) = stream.split();
        *self.writer.lock().await = Some(writer);

        while let Some(frame) = reader.next().await {
            let frame =
                frame.map_err(|err| Error::Adapter(format!("OneBot WebSocket error: {err}")))?;

            let payload = match frame {
                WsMessage::Text(text) => text.to_string(),
                WsMessage::Binary(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
                WsMessage::Close(_) => break,
                _ => continue,
            };

            let json: serde_json::Value = match serde_json::from_str(&payload) {
                Ok(json) => json,
                Err(_) => continue,
            };

            if let Some(echo) = json.get("echo").and_then(|value| value.as_str()) {
                let sender = self.pending.requests.lock().await.remove(echo);
                if let Some(sender) = sender {
                    let result = if json.get("status").and_then(|value| value.as_str())
                        == Some("ok")
                        || json.get("retcode").and_then(|value| value.as_i64()) == Some(0)
                    {
                        Ok(json.get("data").cloned().unwrap_or(serde_json::Value::Null))
                    } else {
                        Err(Error::Adapter(format!("OneBot action failed: {payload}")))
                    };
                    let _ = sender.send(result);
                }
                continue;
            }

            if json.get("post_type").and_then(|value| value.as_str()) == Some("message") {
                if let Some(event) = normalize_message_event(&json) {
                    if events.send(event).await.is_err() {
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    async fn send_message(
        &self,
        channel_id: &str,
        content: &str,
        reply_to: Option<&str>,
    ) -> Result<Message> {
        let (kind, id) = parse_channel_id(channel_id)?;
        let mut segments = serde_json::json!([{ "type": "text", "data": { "text": content } }]);

        if let Some(reply_to) = reply_to {
            segments = serde_json::json!([
                { "type": "reply", "data": { "id": reply_to } },
                { "type": "text", "data": { "text": content } },
            ]);
        }

        let action = match kind {
            ChannelKind::Group => "send_group_msg",
            ChannelKind::Direct => "send_private_msg",
        };

        let params = match kind {
            ChannelKind::Group => serde_json::json!({
                "group_id": id,
                "message": segments,
            }),
            ChannelKind::Direct => serde_json::json!({
                "user_id": id,
                "message": segments,
            }),
        };

        let response = self.send_request(action, params).await?;
        let message_id = response
            .get("message_id")
            .and_then(|value| value.as_i64())
            .ok_or_else(|| Error::Adapter("OneBot response did not contain message_id".into()))?;

        Ok(Message::new(
            message_id.to_string(),
            channel_id.to_owned(),
            self.self_user(),
            content,
        ))
    }
}

enum ChannelKind {
    Group,
    Direct,
}

fn parse_channel_id(channel_id: &str) -> Result<(ChannelKind, i64)> {
    let (kind, id) = channel_id
        .split_once(':')
        .ok_or_else(|| Error::Adapter(format!("invalid OneBot channel ID: {channel_id}")))?;

    let id = id
        .parse()
        .map_err(|_| Error::Adapter(format!("invalid OneBot channel ID: {channel_id}")))?;

    match kind {
        "group" => Ok((ChannelKind::Group, id)),
        "private" => Ok((ChannelKind::Direct, id)),
        _ => Err(Error::Adapter(format!(
            "invalid OneBot channel kind: {kind}"
        ))),
    }
}

fn normalize_message_event(event: &serde_json::Value) -> Option<Event> {
    let message_type = event.get("message_type")?.as_str()?;
    let message_id = event.get("message_id")?.as_i64()?;
    let user_id = event.get("user_id")?.as_i64()?;
    let _raw_message = event.get("raw_message").and_then(|value| value.as_str())?;

    let (channel_id, content, attachments) = match message_type {
        "group" => {
            let group_id = event.get("group_id")?.as_i64()?;
            let (content, attachments) = normalize_message(event.get("message")?);
            (format!("group:{group_id}"), content, attachments)
        }
        "private" => {
            let (content, attachments) = normalize_message(event.get("message")?);
            (format!("private:{user_id}"), content, attachments)
        }
        _ => return None,
    };

    let sender = event
        .get("sender")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let nickname = sender
        .get("nickname")
        .and_then(|value| value.as_str())
        .map(str::to_owned);
    let user_id_string = user_id.to_string();

    let mut message = Message::new(
        message_id.to_string(),
        channel_id,
        User {
            id: user_id_string,
            username: nickname.clone(),
            display_name: nickname,
            is_bot: false,
        },
        content,
    )
    .with_attachments(attachments);

    if let Some(time) = event.get("time").and_then(|value| value.as_i64()) {
        if let Some(timestamp) = UNIX_EPOCH.checked_add(Duration::from_secs(time.unsigned_abs())) {
            message = message.with_timestamp(timestamp);
        }
    }

    Some(Event::Message(message))
}

fn normalize_message(message: &serde_json::Value) -> (String, Vec<Attachment>) {
    match message {
        serde_json::Value::String(raw) => (raw.clone(), Vec::new()),
        serde_json::Value::Array(segments) => {
            let mut content = String::new();
            let mut attachments = Vec::new();

            for segment in segments {
                let kind = segment.get("type").and_then(|value| value.as_str());
                let data = segment
                    .get("data")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);

                match kind {
                    Some("text") => {
                        if let Some(text) = data.get("text").and_then(|value| value.as_str()) {
                            content.push_str(text);
                        }
                    }
                    Some("image") => {
                        let url = data
                            .get("url")
                            .or_else(|| data.get("file"))
                            .and_then(|value| value.as_str())
                            .map(str::to_owned);
                        attachments.push(Attachment {
                            id: url.clone().unwrap_or_else(|| "image".into()),
                            filename: "image".into(),
                            url,
                            content_type: Some("image/*".into()),
                        });
                    }
                    Some("at") => {
                        if let Some(qq) = data.get("qq").and_then(|value| value.as_str()) {
                            content.push_str(&format!("@{qq}"));
                        }
                    }
                    _ => {}
                }
            }

            (content, attachments)
        }
        _ => (String::new(), Vec::new()),
    }
}

trait MessageExt {
    fn with_attachments(self, attachments: Vec<Attachment>) -> Self;
    fn with_timestamp(self, timestamp: SystemTime) -> Self;
}

impl MessageExt for Message {
    fn with_attachments(mut self, attachments: Vec<Attachment>) -> Self {
        self.attachments = attachments;
        self
    }

    fn with_timestamp(mut self, timestamp: SystemTime) -> Self {
        self.timestamp = Some(timestamp);
        self
    }
}

#[allow(dead_code)]
struct Unused<T>(T);

#[allow(dead_code)]
type UnusedChannel = mpsc::Sender<()>;
