use asanagi::{
    Bot, Context, Event, Handler, Result,
    mock::MockAdapter,
    model::{Message, User},
};
use async_trait::async_trait;
use std::time::Duration;

struct Echo;

#[async_trait]
impl Handler for Echo {
    async fn handle(&self, context: Context, event: Event) -> Result<()> {
        if let Event::Message(message) = event {
            context.reply(&message, &message.content).await?;
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let (adapter, handle) = MockAdapter::new("mock");
    let bot = tokio::spawn(async move { Bot::new(adapter).handler(Echo).run().await });

    let message = Message::new("1", "general", User::new("alice"), "hello");
    handle.deliver_message(message).await?;
    tokio::time::sleep(Duration::from_millis(20)).await;
    let sent = handle.sent().await;
    drop(handle);

    let result = tokio::time::timeout(Duration::from_secs(2), bot)
        .await
        .map_err(|_| asanagi::Error::Runtime("runtime timed out".into()))?
        .map_err(|err| asanagi::Error::Runtime(err.to_string()))?;
    result?;

    for message in sent {
        println!("{}: {}", message.author.label(), message.content);
    }

    Ok(())
}
