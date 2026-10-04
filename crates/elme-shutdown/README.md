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

### v0.1.0 (initial release)

- [x] Fix `ProgressWriter` `stdout`/`stderr` merge
- [x] `ProgressWriter` docs
- [ ] Fix `LifecycleStage`/`RunState` incorrect usage of `loaded_command()`
- [ ] Fix double `Reload` not stored to pending
- [ ] Remove tracing events from `ShutdownManager::stop()` & `ShutdownManager::reload()`?
- [ ] Break up `basic` example workers into dedicated modules
