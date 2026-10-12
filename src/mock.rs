use std::{sync::Arc, time::SystemTime};

use async_trait::async_trait;
use tokio::sync::{Mutex, mpsc};

use crate::{
    Error, Result,
    adapter::{Adapter, EventSender},
    model::{Event, Message, User},
};

/// An in-memory adapter useful for examples and tests.
pub struct MockAdapter {
    name: String,
    self_user: User,
    sent: Arc<Mutex<Vec<Message>>>,
    incoming: Mutex<mpsc::Receiver<Event>>,
}

impl MockAdapter {
    /// Create an adapter and a handle for injecting events and inspecting output.
    pub fn new(name: impl Into<String>) -> (Self, MockHandle) {
        let (tx, rx) = mpsc::channel(128);
        let sent = Arc::new(Mutex::new(Vec::new()));

        (
            Self {
                name: name.into(),
                self_user: User::new("mock-bot"),
                sent: sent.clone(),
                incoming: Mutex::new(rx),
            },
            MockHandle { events: tx, sent },
        )
    }
}

#[async_trait]
impl Adapter for MockAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn self_user(&self) -> User {
        self.self_user.clone()
    }

    async fn start(&self, events: EventSender) -> Result<()> {
        let mut incoming = self.incoming.lock().await;

        while let Some(event) = incoming.recv().await {
            if events.send(event).await.is_err() {
                break;
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
        let mut sent = self.sent.lock().await;
        let message = Message {
            id: format!("outgoing-{}", sent.len() + 1),
            channel_id: channel_id.to_owned(),
            author: self.self_user.clone(),
            content: content.to_owned(),
            attachments: Vec::new(),
            timestamp: Some(SystemTime::now()),
            reply_to: reply_to.map(str::to_owned),
        };

        sent.push(message.clone());
        Ok(message)
    }
}

/// Test controller for [`MockAdapter`].
pub struct MockHandle {
    events: mpsc::Sender<Event>,
    sent: Arc<Mutex<Vec<Message>>>,
}

impl MockHandle {
    /// Inject an event into the adapter.
    pub async fn deliver(&self, event: Event) -> Result<()> {
        self.events
            .send(event)
            .await
            .map_err(|_| Error::Runtime("mock adapter was dropped".into()))
    }

    /// Inject a message event.
    pub async fn deliver_message(&self, message: Message) -> Result<()> {
        self.deliver(Event::Message(message)).await
    }

    /// Messages sent by the adapter.
    pub async fn sent(&self) -> Vec<Message> {
        self.sent.lock().await.clone()
    }
}
