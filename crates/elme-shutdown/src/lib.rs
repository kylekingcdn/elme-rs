#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(rustdoc::broken_intra_doc_links)]
#![warn(unreachable_pub)]

#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/kylekingcdn/elme-rs/refs/heads/main/assets/elme-rs.png?raw=true")]

pub mod command;
pub(crate) mod config;
pub(crate) mod manager;
pub(crate) mod signal;
pub(crate) mod state;
pub mod task;
pub(crate) mod teardown;

#[cfg(feature = "progress")]
pub(crate) mod progress;

pub use config::{ShutdownConfig, ShutdownOverrideConfig};
pub use manager::ShutdownManager;
pub use state::{LifecycleStage, RegisterError, RunState};
pub use task::TaskHandle;
pub use teardown::{
    stats::{TeardownStats, TeardownTimeoutStats},
    TeardownResult,
};

#[cfg(all(
    feature = "progress",
    feature = "progress-writer",
))]
#[cfg_attr(docsrs, doc(cfg(
    feature = "progress-writer",
)))]
pub use progress::writer::ProgressWriter;
