<p align='center'>
  <img src='https://raw.githubusercontent.com/kylekingcdn/elme-rs/refs/heads/main/assets/elme-rs.png?raw=true' width=400 />
</p>

# `elme-shutdown`

An `elme` module providing graceful shutdown with a familiar interface (and a ton of QOL).

[**crates.io**](https://crates.io/crates/elme-shutdown)
|
[**Docs**](https://docs.rs/elme-shutdown/latest)
|
[**GitHub**](https://github.com/kylekingcdn/elme-rs/tree/main/crates/elme-shutdown)

## Features

Provides common graceful shutdown features, such as:

- Handling of signals (incl. `ctrl`+`c`)
- Programmatic shutdown trigger
- Futures for awaiting shutdown start or completion
- Graceful shutdown timeout

And additional quality-of-life features, such as:

- Live-reload support via a one-liner (w/ `SIGHUP` trigger)
- Zero-conf task & timeout progress bars (`progress` feature)
- Task/worker identification, allowing for OOTB diagnostics & monitoring
- Teardown summary reports, including breakdown of stopped/timed-out tasks
- Teardown/timeout hooks for custom reporting callbacks
- Exposes app lifecycle states
- Simultaneous support for builder + runtime configuration

## High-level overview

The library module docs contain a succinct briefing of core concepts.

For first-time users or those seeking a quick overview, [this is the place to start](https://docs.rs/elme-shutdown/latest/elme_shutdown/#overview)

## Examples

See the [`examples`](https://github.com/kylekingcdn/elme-rs/tree/main/crates/elme-shutdown/examples) directory for example usage.

## Feature flags

Features for the corresponding crates (`elme`, `elme-shutdown`) are listed below.

**`elme`** | **`elme-shutdown`** | **Description**
-|-|-
**`shutdown`** | *n/a* | Enables this module
**`shutdown-progress`** | **`progress`** | Enables task + timeout progress bars during teardown
**`shutdown-progress-writer`** | **`progress-writer`** | Provides a [`tracing-subscriber`](https://docs.rs/tracing-subscriber/latest) writer for clean `tracing` + `progress` output

## Roadmap

### `0.2.0`

- [x] serde-optional support
- [ ] Full docs for configuration
- [x] Support for disabling progress at run-time
- [ ] Proper signal handling support on Windows
- [ ] Support for attaching arbitrary IDs to each instance of a task
      - Unused internally, but would be a nice QoL bonus w/ `tracing` spans

### Upcoming

- [ ] Pre-defined output formats for teardown log messages, e.g:
      - Log each finished instance and output remaining tasks w/ counts (current behaviour)
      - Log each task once when all of its instances have stopped
      - Log just remaining tasks when upon all instances of a task completing
