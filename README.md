# Asanagi

Asanagi is a platform-agnostic Rust framework for building chat bots. It provides a shared message model, an adapter interface, and a small Tokio-based runtime, so the same handler logic can be reused across platforms instead of being rewritten for each chat service.

The project is motivated by a simple frustration: writing separate bots for every platform is tedious. Asanagi aims to keep the common layers useful while allowing platform-specific behavior to remain visible when it matters.

## Status

Asanagi is an early-stage project. The core runtime and message model are usable, but APIs may change as support for message edits, deletions, reactions, and additional platforms is added.

## Platform support

| Platform | Crate | Status |
| --- | --- | --- |
| Mock/testing | `asanagi` | Stable |
| Discord | `asanagi-discord` | Initial support |
| QQ via OneBot v11 | `asanagi-onebot` | Initial support |
| Telegram | Planned | Not started |
| Matrix | Planned | Not started |
| XMPP | Planned | Not started |

The OneBot adapter targets implementations such as NapCat, Lagrange.OneBot, and LLOneBot.

## Installation

Add the core framework and the adapters you need:

```toml
[dependencies]
asanagi = { path = "../asanagi" }
asanagi-discord = { path = "../asanagi/adapters/discord" }
asanagi-onebot = { path = "../asanagi/adapters/onebot" }
```

Once published, path dependencies can be replaced with crates.io versions.

## Example

This echo bot works with any adapter implementing `asanagi::Adapter`:

```rust
use asanagi::{Context, Event, Handler, Result};
use async_trait::async_trait;

struct Echo;

#[async_trait]
impl Handler for Echo {
    async fn handle(&self, ctx: Context, event: Event) -> Result<()> {
        if let Event::Message(message) = event {
            ctx.reply(&message, &message.content).await?;
        }
        Ok(())
    }
}
```

To run it against Discord:

```rust
use asanagi::Bot;
use asanagi_discord::{DiscordAdapter, DiscordConfig};

#[tokio::main]
async fn main() -> asanagi::Result<()> {
    let config = DiscordConfig::from_env()?;
    Bot::new(DiscordAdapter::new(config)).handler(Echo).run().await
}
```

Or against a OneBot server:

```rust
use asanagi::Bot;
use asanagi_onebot::{OneBotAdapter, OneBotConfig};

#[tokio::main]
async fn main() -> asanagi::Result<()> {
    let config = OneBotConfig::from_env()?;
    Bot::new(OneBotAdapter::new(config)).handler(Echo).run().await
}
```

A runnable mock example is available at `examples/mock_echo.rs`.

## Configuration

Adapters are configured through environment variables:

- `DISCORD_TOKEN` — Discord bot token
- `ONEBOT_WS_URL` — OneBot WebSocket endpoint, e.g. `ws://localhost:3001`
- `ONEBOT_ACCESS_TOKEN` — Optional OneBot access token

Keep tokens out of source control and load them from your shell, `.env` file, or secret manager.

## Development

The repository is a Cargo workspace:

- `src/` — core framework, shared model, and runtime
- `adapters/discord/` — Discord adapter
- `adapters/onebot/` — OneBot v11 adapter
- `examples/` — runnable examples
- `tests/` — integration tests

Useful commands:

```sh
cargo fmt
cargo test --workspace
cargo doc --no-deps
```

## License

MIT. See [LICENSE](LICENSE).
