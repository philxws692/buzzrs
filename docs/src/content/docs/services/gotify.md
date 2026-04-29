---
title: Gotify
---

## Gotify

Send notifications to a [Gotify](https://gotify.net) server via the push message API.

**URL format:** `gotify://<host>[:<port>]/<token>`

## Query parameters

| Parameter | Type | Default | Description |
| --------- | ---- | ------- | ----------- |
| `title` | string | _(empty)_ | Message title shown in the Gotify client. |
| `priority` | integer | `0` | Message priority sent to Gotify. |
| `disable_tls` | bool | `false` | Use plain HTTP instead of HTTPS. |

## Examples

### Minimal — message only

```rust
use buzzrs::buzz;

#[tokio::main]
async fn main() {
    buzz!("gotify://push.example.com/AszT3Hxyz", "Deployment finished.");
}
```

### `title` — set a message title

```rust
buzz!(
    "gotify://push.example.com/AszT3Hxyz?title=Deployment",
    "Service restarted successfully."
);
```

### `priority` — set message priority

```rust
buzz!(
    "gotify://push.example.com/AszT3Hxyz?priority=7",
    "Disk usage above 90%."
);
```

### `disable_tls` — use HTTP instead of HTTPS

```rust
buzz!(
    "gotify://internal.lan:8080/AszT3Hxyz?disable_tls=true",
    "Internal alert."
);
```

### All parameters combined

```rust
buzz!(
    "gotify://push.example.com/AszT3Hxyz?title=Ops+Alert&priority=8&disable_tls=false",
    "Database backup completed."
);
```

### Lower-level API

```rust
use buzzrs::Buzz;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let buzz = Buzz::new();
    let service = buzz.build_service(
        "gotify://push.example.com/AszT3Hxyz?title=Ops+Alert&priority=8",
    )?;
    service.send("Database backup completed.").await?;
    Ok(())
}
```

### Sync (requires `no_async` feature)

```rust
use buzzrs::buzz_sync;

fn main() {
    buzz_sync!(
        "gotify://push.example.com/AszT3Hxyz?title=Ops+Alert&priority=5",
        "Cron job finished."
    );
}
```
