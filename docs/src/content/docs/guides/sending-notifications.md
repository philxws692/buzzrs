---
title: Sending Notifications
description: Learn how to send notifications from buzzrs
---

## Sending notifications
To use `buzzrs` now in your project, just write
```rs
use buzzrs::buzz;

#[tokio::main]
async fn main() {
    buzz!("ntfy://buzzrs:bzzz@ntfy.sh/important", "buzzrs is awesome!")
}
```

For services see [Services](/services/overview)
