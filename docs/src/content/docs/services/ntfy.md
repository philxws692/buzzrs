---
title: ntfy
---

## ntfy

Send notifications to an [ntfy](https://ntfy.sh) server via the publish API.

**URL format:** `ntfy://<host>[:<port>]/<token>/<topic>`

## Query parameters

| Parameter | Aliases | Type | Default | Description |
| --------- | ------- | ---- | ------- | ----------- |
| `title` | | string | _(empty)_ | Notification title. |
| `priority` | | integer (1–5) | `0` | Notification priority. 1 = min, 3 = default, 5 = max. |
| `tags` | | string | _(empty)_ | Comma-separated list of tags or emoji shortcodes (e.g. `warning,cd`). |
| `cache` | | bool | `true` | Enable server-side message caching. Set to `false` to disable. |
| `click` | | string (URL) | _(empty)_ | URL to open when the notification is clicked. |
| `attach` | | string (URL) | _(empty)_ | URL of a file to attach to the notification. |
| `filename` | | string | _(empty)_ | Filename override for the attached file. |
| `icon` | | string (URL) | _(empty)_ | URL of an icon image to display with the notification. |
| `delay` | `in`, `at` | string | _(empty)_ | Schedule delivery: duration (e.g. `30m`, `2h`), timestamp, or natural language (e.g. `tomorrow`). |
| `email` | | string | _(empty)_ | Forward the notification to this e-mail address. |
| `disable_tls` | | bool | `false` | Use plain HTTP instead of HTTPS. |

## Examples

### Minimal — message only

```rust
use buzzrs::buzz;

#[tokio::main]
async fn main() {
    buzz!(
        "ntfy://ntfy.sh/tk_abc123/my-topic",
        "Deployment finished."
    );
}
```

### `title` — set a notification title

```rust
buzz!(
    "ntfy://ntfy.sh/tk_abc123/my-topic?title=Deployment",
    "Build #42 succeeded."
);
```

### `priority` — set notification priority

```rust
buzz!(
    "ntfy://ntfy.sh/tk_abc123/my-topic?priority=5",
    "Disk usage above 90%."
);
```

### `tags` — attach tags or emoji shortcodes

```rust
// single tag
buzz!(
    "ntfy://ntfy.sh/tk_abc123/my-topic?tags=warning",
    "High memory usage."
);

// multiple tags, comma-separated
buzz!(
    "ntfy://ntfy.sh/tk_abc123/my-topic?tags=warning,cd",
    "High memory usage."
);
```

### `cache` — disable server-side caching

```rust
buzz!(
    "ntfy://ntfy.sh/tk_abc123/my-topic?cache=false",
    "One-time alert, do not cache."
);
```

### `click` — open a URL when the notification is tapped

```rust
buzz!(
    "ntfy://ntfy.sh/tk_abc123/my-topic?click=https%3A%2F%2Fexample.com%2Fdashboard",
    "New deployment — tap to open dashboard."
);
```

### `attach` — attach a file from a URL

```rust
buzz!(
    "ntfy://ntfy.sh/tk_abc123/my-topic?attach=https%3A%2F%2Fexample.com%2Freport.pdf",
    "Monthly report is ready."
);
```

### `filename` — override the attachment filename

```rust
buzz!(
    "ntfy://ntfy.sh/tk_abc123/my-topic\
     ?attach=https%3A%2F%2Fexample.com%2Freport.pdf\
     &filename=june-report.pdf",
    "Monthly report is ready."
);
```

### `icon` — set the notification icon

```rust
buzz!(
    "ntfy://ntfy.sh/tk_abc123/my-topic?icon=https%3A%2F%2Fexample.com%2Ficon.png",
    "Backup completed."
);
```

### `delay` / `in` / `at` — schedule delivery

```rust
// delay by duration
buzz!(
    "ntfy://ntfy.sh/tk_abc123/my-topic?delay=30m",
    "Reminder: review the logs."
);

// using the `in` alias
buzz!(
    "ntfy://ntfy.sh/tk_abc123/my-topic?in=2h",
    "Reminder: review the logs."
);

// using the `at` alias with natural language
buzz!(
    "ntfy://ntfy.sh/tk_abc123/my-topic?at=tomorrow",
    "Good morning reminder."
);
```

### `email` — forward the notification by e-mail

```rust
buzz!(
    "ntfy://ntfy.sh/tk_abc123/my-topic?email=ops%40example.com",
    "Critical error in production."
);
```

### `disable_tls` — use HTTP instead of HTTPS

```rust
buzz!(
    "ntfy://internal.lan:8080/tk_abc123/my-topic?disable_tls=true",
    "Internal network alert."
);
```

### All parameters combined

```rust
buzz!(
    "ntfy://ntfy.sh/tk_abc123/my-topic\
     ?title=Monthly+Backup\
     &priority=4\
     &tags=warning,backup\
     &cache=false\
     &click=https%3A%2F%2Fexample.com%2Fdashboard\
     &attach=https%3A%2F%2Fexample.com%2Fbackup.tar.gz\
     &filename=backup-2024-06.tar.gz\
     &icon=https%3A%2F%2Fexample.com%2Ficon.png\
     &delay=10m\
     &email=ops%40example.com",
    "Backup snapshot created successfully."
);
```

### Lower-level API

```rust
use buzzrs::Buzz;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let buzz = Buzz::new();
    let service = buzz.build_service(
        "ntfy://ntfy.sh/tk_abc123/my-topic?title=Alert&priority=5&tags=warning",
    )?;
    service.send("Disk usage above 90%.").await?;
    Ok(())
}
```

### Sync (requires `no_async` feature)

```rust
use buzzrs::buzz_sync;

fn main() {
    buzz_sync!(
        "ntfy://ntfy.sh/tk_abc123/my-topic?title=Cron&priority=3",
        "Nightly job completed."
    );
}
```
