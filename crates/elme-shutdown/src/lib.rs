#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(rustdoc::broken_intra_doc_links)]
// #![warn(unreachable_pub)]

#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/kylekingcdn/elme-rs/refs/heads/main/assets/elme-rs.png?raw=true")]

pub mod command;
pub mod config;
pub mod manager;
pub(crate) mod signal;
pub(crate) mod state;
pub mod task;
pub mod teardown;

#[cfg(feature = "progress")]
pub(crate) mod progress;

pub use command::{Command, CommandResult, ReloadResult, StopCommand, StopResult};
pub use config::{ShutdownConfig, ShutdownOverrideConfig};
pub use manager::ShutdownManager;
pub use state::{LifecycleStage, RegisterError, RunState};
pub use task::{
    InstanceCount,
    TaskHandle,
    TaskList,
    TrackedTaskList,
    TransitioningTaskList,
    TransitionInstanceCount,
};
pub use teardown::stats::{TeardownStats, TeardownTimeoutStats};

#[cfg(all(
    feature = "progress",
    feature = "progress-writer",
))]
#[cfg_attr(docsrs, doc(cfg(all(
    feature = "progress",
    feature = "progress-writer",
))))]
pub use progress::writer::ProgressWriter;
