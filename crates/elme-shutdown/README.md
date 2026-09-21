<p align='center'>
  <img src='https://raw.githubusercontent.com/kylekingcdn/elme-rs/refs/heads/main/assets/elme-rs.png?raw=true' width=400 />
</p>

# `elme-shutdown`

An `elme` module providing graceful shutdown with a familiar interface (and a ton of QOL).

> **Warning**
>
> Full documentation will be published soon

[**crates.io**](https://crates.io/crates/elme-shutdown)
|
[**Docs**](https://docs.rs/elme-shutdown/latest)
|
[**GitHub**](https://github.com/kylekingcdn/elme-rs/tree/main/crates/elme-shutdown)

## Features

Provides common graceful shutdown features, such as:

- Signal + ctrl+c handling
- Programmatic shutdown trigger
- Futures for awaiting shutdown start or completion
- Graceful shutdown tomeout

And additional quality-of-life features, such as:

- Live-reload support via a one-liner (w/ `SIGHUP` trigger)
- Zero-conf task & timeout progress bars (`progress` feature)
- Task/worker identification, allowing for dead-simple diagnostics & monitoring
- Teardown summary reports, including breakdown of stopped/timed-out tasks
- Teardown/timeout hooks for custom reporting callbacks
- Exposes app lifecycle states
- Supports builder + runtime configuration (simultaneously)

## Demo

> coming soon

## Examples

See the [`examples`](https://github.com/kylekingcdn/elme-rs/tree/main/crates/elme-shutdown/examples) directory for example usage.

## Feature flags

Features for the corresponding crate (`elme`, `elme-shutdown`) are listed below.

**`elme`** | **`elme-shutdown`** | **Description**
-|-|-
**`shutdown`** | *n/a* | Enables this module
**`shutdown-progress`** | **`progress`** | Enables task + timeout progress bars during teardown
**`shutdown-progress-writer`** | **`progress-writer`** | Provides a [`tracing-subscriber`](https://docs.rs/tracing-subscriber/latest) writer for clean `tracing` + `progress` output