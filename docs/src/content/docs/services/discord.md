---
title: Discord
---

## Discord

Send notifications to a [Discord](https://discord.com) channel via an incoming webhook.

**URL format:** `discord://<token>@<webhook_id>`

The token and webhook ID come from the webhook URL Discord generates:
`https://discord.com/api/webhooks/<webhook_id>/<token>`

## Query parameters

| Parameter | Alias | Type | Default | Description |
| --------- | ----- | ---- | ------- | ----------- |
| `username` | | string | `Buzz` | Display name shown as the webhook sender. |
| `avatar` | `avatarurl` | string | _(buzz default avatar)_ | URL of the avatar image shown next to the message. |
| `title` | | string | `New notification from Buzz` | Embed title. Setting any embed parameter enables the embed. |
| `color` | | string | `FF9900` | Embed accent color as a hex string (e.g. `FF9900` or `0xFF9900`). |
| `base_url` | `baseurl` | string | `https://discord.com` | Base URL for the webhook request. Primarily useful for testing. |

### Embed behavior

When all of `username`, `avatar`, `title`, and `color` are left at their defaults, the message is sent as plain content with no embed. Changing any one of those parameters causes the message body to be sent inside a Discord embed instead.

## Examples

### Minimal — plain message, no embed

```rust
use buzzrs::buzz;

#[tokio::main]
async fn main() {
    buzz!(
        "discord://KkoZlPa0cAVJRBa7kMuUEFff7zXDYgNDPk@1234567890123456789",
        "Deployment finished."
    );
}
```

### `username` — override the sender display name

```rust
buzz!(
    "discord://KkoZlPa0cAVJRBa7kMuUEFff7zXDYgNDPk@1234567890123456789?username=Ops",
    "Service restarted."
);
```

### `avatar` / `avatarurl` — override the sender avatar

```rust
// using `avatar`
buzz!(
    "discord://KkoZlPa0cAVJRBa7kMuUEFff7zXDYgNDPk@1234567890123456789\
     ?avatar=https%3A%2F%2Fexample.com%2Fbot.png",
    "Service restarted."
);

// using the `avatarurl` alias
buzz!(
    "discord://KkoZlPa0cAVJRBa7kMuUEFff7zXDYgNDPk@1234567890123456789\
     ?avatarurl=https%3A%2F%2Fexample.com%2Fbot.png",
    "Service restarted."
);
```

### `title` — set the embed title

```rust
buzz!(
    "discord://KkoZlPa0cAVJRBa7kMuUEFff7zXDYgNDPk@1234567890123456789?title=Deployment",
    "Build #42 succeeded."
);
```

### `color` — set the embed accent color

```rust
// green embed
buzz!(
    "discord://KkoZlPa0cAVJRBa7kMuUEFff7zXDYgNDPk@1234567890123456789?color=00FF00",
    "All checks passed."
);

// using `0x` prefix
buzz!(
    "discord://KkoZlPa0cAVJRBa7kMuUEFff7zXDYgNDPk@1234567890123456789?color=0x00FF00",
    "All checks passed."
);
```

### All parameters combined

```rust
buzz!(
    "discord://KkoZlPa0cAVJRBa7kMuUEFff7zXDYgNDPk@1234567890123456789\
     ?username=Ops\
     &avatar=https%3A%2F%2Fexample.com%2Fbot.png\
     &title=Deployment+Alert\
     &color=0x8F7AAF",
    "Production deployment completed."
);
```

### Lower-level API

```rust
use buzzrs::Buzz;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let buzz = Buzz::new();
    let service = buzz.build_service(
        "discord://KkoZlPa0cAVJRBa7kMuUEFff7zXDYgNDPk@1234567890123456789\
         ?username=Ops&title=Deployment+Alert&color=0x8F7AAF",
    )?;
    service.send("Production deployment completed.").await?;
    Ok(())
}
```

### Sync (requires `no_async` feature)

```rust
use buzzrs::buzz_sync;

fn main() {
    buzz_sync!(
        "discord://KkoZlPa0cAVJRBa7kMuUEFff7zXDYgNDPk@1234567890123456789?title=Alert",
        "Cron job finished."
    );
}
```
