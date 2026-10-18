use lib_conf::{
    LibConfig,
    adapter::{
        duration::SecondsAdapter,
        tracing_level::TracingLevelAdapter,
    }
};
use std::time::Duration;

/// User configuration for `elme-shutdown`
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Copy, Clone, LibConfig)]
#[config(example_field_name = "shutdown")]
pub struct ShutdownConfig {
    /// The time to wait for tasks to gracefully stop.
    ///
    /// Once the timeout is reached, the application is immediately
    /// terminated (termination applies to both Stop and Reload commands)
    #[config(
        copy, default = Duration::from_secs(10),
        override_from = u64, override_via = SecondsAdapter,
        override_from_copy,
    )]
    pub(crate) timeout: Duration,

    /// Whether support for signal handling is enabled.
    ///
    /// # Platform support
    /// 
    /// Signal handling is platform-dependent. Associated behaviours are outlined below.
    ///
    /// ## Unix
    ///
    /// Enables handling of `INT`, `TERM`, `QUIT`, and `HUP` signals.
    ///
    /// ### Behavior
    ///
    /// - **`QUIT`**: Terminates immediately, without gracefully stopping tasks.
    /// - **`INT`/`TERM`**: Triggers graceful shutdown. Identical to calling [`ShutdownManager::stop()`](crate::ShutdownManager::stop).
    /// - **`HUP`**: When [`reload_support`](Self::reload_support)
    ///   - is ***enabled*** (default), this is identical to calling [`ShutdownManager::reload()`](crate::ShutdownManager::reload).
    ///   - is ***disabled***, this is identical to calling [`ShutdownManager::stop()`](crate::ShutdownManager::stop).
    ///
    /// ## Windows
    ///
    /// Enables handling of the following [`CTRL` events](https://learn.microsoft.com/en-us/windows/console/setconsolectrlhandler):
    /// - `C`, `BREAK`, `CLOSE`, `LOGOFF`, `SHUTDOWN`.
    ///
    /// ### Behavior
    ///
    /// By default, all events share the same behavior:
    ///
    /// - Triggers graceful shutdown. Identical to calling [`ShutdownManager::stop()`](crate::ShutdownManager::stop).
    ///
    /// ### Triggering `Reload`
    ///
    /// Windows does not have an event equivalent to `SIGHUP`. Therefore, by default, a `Reload` command can only be triggered programmatically
    /// via [`ShutdownManager::reload()`](crate::ShutdownManager::reload). 
    ///
    /// For `SIGHUP`-like behavior, [`reload_on_ctrl_break`](Self::reload_on_ctrl_break) can be enabled to issue `Reload`
    /// upon `CTRL`+`BREAK`, as opposed to the default of issuing `Stop`.
    ///
    /// # Multiple invocations
    ///
    /// By default, when a `Stop`-issuing signal is a 2nd time, the application terminates immediately.
    ///
    /// See [`terminate_on_second_signal`](Self::terminate_on_second_signal) for more information.
    #[cfg(any(unix, windows))]
    #[cfg_attr(docsrs, doc(cfg(any(unix, windows))))]
    #[config(copy, default = true)]
    pub(crate) handle_signals: bool,

    /// Whether support is enabled for immediate program termination upon receiving a `Stop`-issuing signal more than once.
    ///
    /// - Has no effect if [`handle_signals`](Self::handle_signals) has been disabled.
    ///
    /// See [`handle_signals`](Self::handle_signals) for a list of `Stop`-issuing signals.
    #[cfg(any(unix, windows))]
    #[cfg_attr(docsrs, doc(cfg(any(unix, windows))))]
    #[config(copy, default = true)]
    pub(crate) terminate_on_second_signal: bool,
    
    /// Whether support for issuing `Reload` upon receiving a Windows `CTRL`+`BREAK` signal is enabled.
    ///
    /// - Has no effect if [`handle_signals`](Self::handle_signals) has been disabled.
    /// - Has no effect if [`reload_support`](Self::reload_support) has been disabled (`BREAK` will continue to issue `Stop`).
    ///
    /// **Note:** This option is only exposed on Windows.
    #[cfg(any(docsrs, windows))]
    #[cfg_attr(docsrs, doc(cfg(windows)))]
    #[config(copy, default = false)]
    pub(crate) reload_on_ctrl_break: bool,

    /// Whether support is enabled for live app reloading.
    ///
    /// Issuing a `Reload` will perform the exact same teardown sequence as `Stop`.
    /// However, once teardown is complete - instead of directing the application to exit -
    /// the control-flow will once again begin the startup and run stages.
    ///
    /// A reload can be invoked via [`ShutdownManager::reload()`](crate::ShutdownManager::reload), or by sending the process a `HUP` signal
    /// (on Unix) when [`handle_signals`](Self::handle_signals()) is enabled.
    ///
    /// **If disabled:**
    /// - Calls to [`ShutdownManager::reload()`](crate::ShutdownManager::reload) will be ignored
    /// - Any `Reload`-issuing signals will instead issue `Stop`
    #[config(copy, default = true)]
    pub(crate) reload_support: bool,

    /// Provides config-level support for disabling progress support
    ///
    /// This can be useful in scenarios such as:
    /// - disabling support at runtime via [`ShutdownOverrideConfig`]
    /// - multi-binary workspaces, where additive dependency rules cause undesired enablement
    #[cfg(feature = "progress")]
    #[cfg_attr(docsrs, doc(cfg(feature = "progress")))]
    #[config(copy, default = true)]
    pub(crate) progress_support: bool,

    /// Enables logging of remaining teardown tasks.
    #[config(copy, default = true)]
    pub(crate) log_teardown_remaining: bool,

    /// The [`tracing::Level`] used for the 'remaining tasks' teardown log messages.
    ///
    /// Has no effect if messages have been disabled ([`log_teardown_remaining`] set to`false`).
    #[config(
        copy, default = tracing::Level::INFO,
        override_from = String, override_via = TracingLevelAdapter,
    )]
    pub(crate) log_teardown_remaining_level: tracing::Level,

    /// Enables logging of successful teardown stats.
    #[config(copy, default = true)]
    pub(crate) log_teardown_stats: bool,

    /// The [`tracing::Level`] used for the teardown stats report.
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

    /// The [`tracing::Level`] used for the timed-out teardown stats report.
    ///
    /// Has no effect if timeout stats messages have been disabled ([`log_timeout_stats`] set to`false`).
    #[config(
        copy, default = tracing::Level::ERROR,
        override_from = String, override_via = TracingLevelAdapter,
    )]
    pub(crate) log_timeout_stats_level: tracing::Level,
}
