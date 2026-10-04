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
//! # Feature flags
//!
#![doc = include_str!("../doc/feature_flags.md")]
//!
//! # Overview
//!
#![doc = include_str!("../doc/overview.md")]

/// Types relating to the set of supported commands
pub mod command;
mod config;
mod manager;
mod signal;
mod state;
/// Types relating to tasks and task instances
pub mod task;
mod teardown;

#[cfg(feature = "progress")]
mod progress;

pub use config::{ShutdownConfig, ShutdownConfigBuilder, ShutdownOverrideConfig};
pub use manager::ShutdownManager;
pub use state::{InformStartingError, InformStartedError, LifecycleStage, RunState};
pub use task::TaskHandle;
pub use teardown::stats::{TeardownStats, TeardownTimeoutStats};

#[cfg(feature = "progress-writer")]
#[cfg_attr(docsrs, doc(cfg(feature = "progress-writer")))]
pub use progress::writer::ProgressWriter;
