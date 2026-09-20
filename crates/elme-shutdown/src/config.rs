use lib_conf::{LibConfig, adapter::{
    duration::SecondsAdapter,
    tracing_level::TracingLevelAdapter,
}};
use std::time::Duration;

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Copy, Clone, LibConfig)]
pub struct ShutdownConfig {
    /// The time to wait for tasks to gracefully stop.
    ///
    /// Once the timeout is reached, the application is immediately
    /// terminated (termination applies to both Stop and Reload commands)
    #[config(
        copy, default = Duration::from_secs(10),
        override_from = u64, override_via = SecondsAdapter,
    )]
    pub(crate) timeout: Duration,

    /// Whether support is enabled for the handling of `INT`, `TERM`, and `HUP` signals .
    ///
    /// Signals:
    /// - `INT`/`TERM`: Triggers graceful shutdown. Identical to calling [`ShutdownManager::stop()`](crate::ShutdownManager::stop).
    /// - `HUP`:
    ///   - If reload support is *enabled*, this is identical to calling [`ShutdownManager::reload()`](crate::ShutdownManager::reload).
    ///   - If reload support is *disabled*, this is identical to `INT`/`TERM` handling.
    ///
    /// ## Multiple invocations
    ///
    /// By default, the application will terminate immediately if a given signal is received more than once.
    /// This behaviour can be controlled via [`terminate_on_second_signal`](Self::terminate_on_second_signal).
    ///
    /// <div class="warning">
    /// This applies to `HUP` signals <b>only if reload support is disabled</b>.<br>
    /// If reload is enabled, only `INT` and `TERM` are affected.
    /// </div>
    #[config(copy, default = true)]
    pub(crate) handle_signals: bool,

    /// Whether support is enabled for immediate program termination when a given signal is received more than once.
    ///
    /// <div class="warning">
    /// This applies to `HUP` signals <b>only if reload support is disabled</b>.<br>
    /// If reload is enabled, only `INT` and `TERM` are affected.
    /// </div>
    #[config(copy, default = true)]
    pub(crate) terminate_on_second_signal: bool,

    /// Whether support is enabled for live app reloading.
    ///
    /// When configured, a reload will perform the exact same teardown sequence as a stop.
    /// However once teardown is complete, instead of directing the application to exit,
    /// it will instead provide the flow back to app re-init and run.
    ///
    /// A reload can be invoked via [`ShutdownManager::reload()`](crate::ShutdownManager::reload) or by sending the process a `HUP` signal
    /// (when [`handle_signals`](Self::handle_signals()) is enabled).
    ///
    /// If disabled, `HUP` signals are handled identically to `INT` and `TERM`.
    /// Calls to [`ShutdownManager::reload()`](crate::ShutdownManager::reload) will be silently ignored.
    #[config(copy, default = true)]
    pub(crate) reload_enabled: bool,

    /// Enables logging of remaining teardown tasks.
    #[config(copy, default = true)]
    pub(crate) teardown_log_remaining: bool,

    /// The `tracing::Level` used for the 'remaining tasks' teardown log messages.
    ///
    /// Has no effect if messages have been disabled ([`teardown_log_remaining`] set to`false`).
    #[config(
        copy, default = tracing::Level::INFO,
        override_from = String, override_via = TracingLevelAdapter,
    )]
    pub(crate) teardown_log_remaining_level: tracing::Level,

    /// Enables logging of successful teardown stats.
    #[config(copy, default = true)]
    pub(crate) log_teardown_stats: bool,

    /// The `tracing::Level` used for the teardown stats report.
    ///
    /// Has no effect if stats messages have been disabled ([`log_teardown_stats`](Self::log_teardown_stats) set to`false`).
    #[config(
        copy, default = tracing::Level::INFO,
        override_from = String, override_via = TracingLevelAdapter,
    )]
    pub(crate) log_teardown_stats_level: tracing::Level,

    /// Enables logging of timed-out teardown stats.
    #[config(copy, default = true)]
    pub(crate) log_timeout_stats: bool,

    /// The `tracing::Level` used for the timed-out teardown stats report.
    ///
    /// Has no effect if timeout stats messages have been disabled ([`log_timeout_stats`] set to`false`).
    #[config(
        copy, default = tracing::Level::ERROR,
        override_from = String, override_via = TracingLevelAdapter,
    )]
    pub(crate) log_timeout_stats_level: tracing::Level,
}
