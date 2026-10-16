#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(rustdoc::broken_intra_doc_links)]
#![warn(unreachable_pub)]
#![warn(missing_docs)]

//#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/kylekingcdn/elme-rs/refs/heads/main/assets/elme-rs.png?raw=true")]

//! # Quick start
//!
//! These are the basic steps taken to setup `elme-shutdown`.
//!
//! For a high-level overview of each component as well as the core concepts, see the
//! [Overview](#overview) section.
//!
#![doc = include_str!("../doc/quick_start.md")]
//!
// hacky workaround for retaining doc compilation without all features
#![cfg_attr(
    feature = "progress-writer",
    doc = " [`mapped_progress_writer()`]: ShutdownManager::mapped_progress_writer"
)]
#![cfg_attr(
    not(feature = "progress-writer"),
    doc = " [`mapped_progress_writer()`]: https://docs.rs/elme-shutdown/latest/elme_shutdown/struct.ShutdownManager.html#method.mapped_progress_writer"
)]
#![cfg_attr(
    feature = "progress-writer",
    doc = " [`tracing_subscriber::fmt`]: mod@tracing_subscriber::fmt"
)]
#![cfg_attr(
    not(feature = "progress-writer"),
    doc = " [`mapped_progress_writer()`]: https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/index.html"
)]
//!
//! # Feature flags
//!
#![doc = include_str!("../doc/feature_flags.md")]
//!
//! # Overview
//!
#![doc = include_str!("../doc/overview.md")]
//!
// hacky workaround for retaining doc compilation without all features
#![cfg_attr(
    feature = "progress-writer",
    doc = " [`mapped_progress_writer()`]: ShutdownManager::mapped_progress_writer"
)]
#![cfg_attr(
    not(feature = "progress-writer"),
    doc = " [`mapped_progress_writer()`]: https://docs.rs/elme-shutdown/latest/elme_shutdown/struct.ShutdownManager.html#method.mapped_progress_writer"
)]
#![cfg_attr(
    feature = "progress-writer",
    doc = " [`ProgressWriter`]: progress::ProgressWriter"
)]
#![cfg_attr(
    not(feature = "progress-writer"),
    doc = " [`ProgressWriter`]: https://docs.rs/elme-shutdown/latest/elme_shutdown/struct.ProgressWriter.html"
)]

/// Types relating to the set of supported commands
pub mod command;
/// Types used to configure `elme-shutdown`
pub mod config;
mod manager;
mod signal;
mod state;
/// Types relating to tasks and task instances
pub mod task;
/// Teardown reports, stats, etc.
pub mod teardown;

/// Progress indicator support for task teardown
#[cfg(feature = "progress")]
#[cfg_attr(docsrs, doc(cfg(feature = "progress")))]
pub mod progress;

pub use config::{ShutdownConfig, ShutdownOverrideConfig};
pub use manager::ShutdownManager;
pub use state::{InformStartingError, InformStartedError, LifecycleStage, RunState};
pub use task::TaskHandle;

