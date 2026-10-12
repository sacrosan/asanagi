use async_trait::async_trait;

use crate::{Result, context::Context, model::Event};

/// Application logic that receives normalized adapter events.
///
/// Handlers are invoked sequentially in registration order. Returning an error
/// stops the bot so bugs are not silently ignored.
#[async_trait]
pub trait Handler: Send + Sync {
    async fn handle(&self, context: Context, event: Event) -> Result<()>;
}
