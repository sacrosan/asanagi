use std::sync::Arc;

use tokio::{sync::mpsc, task::JoinHandle};

use crate::{
    Result,
    adapter::{Adapter, EventSender},
    context::Context,
    error::Error,
    handler::Handler,
    model::Event,
};

/// Runtime that connects an [`Adapter`] to one or more [`Handler`]s.
pub struct Bot {
    adapter: Arc<dyn Adapter>,
    handlers: Vec<Arc<dyn Handler>>,
}

impl Bot {
    pub fn new(adapter: impl Adapter + 'static) -> Self {
        Self {
            adapter: Arc::new(adapter),
            handlers: Vec::new(),
        }
    }

    /// Register a handler. Handlers run in registration order.
    pub fn handler(mut self, handler: impl Handler + 'static) -> Self {
        self.handlers.push(Arc::new(handler));
        self
    }

    /// Start the adapter and process events until it disconnects.
    ///
    /// The first handler error is returned to the caller and the adapter task
    /// is aborted.
    pub async fn run(&self) -> Result<()> {
        let (event_tx, mut event_rx) = mpsc::channel::<Event>(128);
        let task = spawn_adapter(self.adapter.clone(), event_tx);

        while let Some(event) = event_rx.recv().await {
            for handler in &self.handlers {
                let context = Context::new(self.adapter.clone());
                handler.handle(context, event.clone()).await?;
            }
        }

        finish_adapter(task).await
    }
}

fn spawn_adapter(adapter: Arc<dyn Adapter>, events: EventSender) -> JoinHandle<Result<()>> {
    tokio::spawn(async move { adapter.start(events).await })
}

async fn finish_adapter(task: JoinHandle<Result<()>>) -> Result<()> {
    match task.await {
        Ok(result) => result,
        Err(err) if err.is_cancelled() => Ok(()),
        Err(err) => Err(Error::Runtime(format!("adapter task failed: {err}"))),
    }
}
