use asanagi::{
    Bot, Context, Event, Handler, Result,
    mock::MockAdapter,
    model::{Message, User},
};
use async_trait::async_trait;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

struct Echo {
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl Handler for Echo {
    async fn handle(&self, context: Context, event: Event) -> Result<()> {
        self.calls.fetch_add(1, Ordering::SeqCst);

        if let Event::Message(message) = event {
            context.reply(&message, &message.content).await?;
        }

        Ok(())
    }
}

#[tokio::test]
async fn dispatches_messages_and_supports_replies() {
    let (adapter, handle) = MockAdapter::new("test");
    let calls = Arc::new(AtomicUsize::new(0));
    let bot = Bot::new(adapter).handler(Echo {
        calls: calls.clone(),
    });

    let runtime = tokio::spawn(async move { bot.run().await });
    handle
        .deliver(Event::Connected(User::new("bot")))
        .await
        .unwrap();

    let message = Message::new("1", "channel", User::new("alice"), "hello");
    handle.deliver_message(message).await.unwrap();

    let runtime = tokio::spawn(async move {
        runtime.await.unwrap().unwrap();
    });

    // Give the runtime a chance to process the queued event before dropping
    // the final event source.
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    let sent = handle.sent().await;
    drop(handle);
    runtime.await.unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].content, "hello");
    assert_eq!(sent[0].channel_id, "channel");
}
