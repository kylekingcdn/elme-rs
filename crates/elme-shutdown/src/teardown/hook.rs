use crate::{
    config::ShutdownConfig,
    task::map::TransitioningTaskMap,
    teardown::stats::{TeardownStats, TeardownTimeoutStats},
};

#[cfg(feature = "progress")]
use crate::progress::TeardownProgressHook;

use chrono::{DateTime, Utc};
use console::Style;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;
use tracing::Level;

// !- Hook trait

#[allow(unused_variables)]
pub(crate) trait TeardownHook {
    fn on_task_unregistered(
        &self,
        transition_map: &TransitioningTaskMap,
        task_name: &'static str,
    ) {}

    fn on_finished(&self, stats: &TeardownStats) {}

    fn on_timeout(&self, stats: &TeardownTimeoutStats) {}
}

// !- Hook dispatcher

/// Provides persistent storage of hook dependencies
///
/// The [`HookDispatcher`] is built on-the-fly for each teardown call.
/// Therefore, a dedicated struct is required for hooks that require persistent data.
#[derive(Debug, Clone)]
pub(crate) struct HookDeps {
    pub(crate) options: ShutdownConfig,

    #[cfg(feature = "progress")]
    pub(crate) progress_bars: indicatif::MultiProgress,

    pub(crate) callbacks: TeardownCallbacks,
}
impl HookDeps {
    pub(crate) fn new(
        options: ShutdownConfig,
        #[cfg(feature = "progress")]
        progress_bars: indicatif::MultiProgress,
    ) -> Self {
        Self {
            options,
            #[cfg(feature = "progress")]
            progress_bars,
            callbacks: TeardownCallbacks::default(),
        }
    }
}

/// Propagates updates to 'hooks'
///
/// hooks are components that require notifying on teardown progress/state updates.
///
/// <div class="warning">
/// <b>Hooks cannot mutate any state outside of their scope/fields.</b><br>
/// <br>
/// Therefore, hooks are only suitable for integrating with some other component.<br>
/// <ul><li>E.g. logging each task as its unregistered, or providing progress UI.</li></ul>
/// </div>
///
/// The purpose of aggregating hook dispatch here is to avoid polluting the [`UnregisterHandler`]
/// with unrelated fn calls and feature gate blocks
pub(crate) struct HookDispatcher {
    log: TeardownLogHook,
    callbacks: TeardownCallbackHook,

    #[cfg(feature = "progress")]
    progress: Option<TeardownProgressHook>,
}
impl HookDispatcher {
    #[allow(clippy::needless_pass_by_value, unused_variables)] // feature-dependent
    pub(crate) fn new(
        transition_map: &TransitioningTaskMap,
        started_at: DateTime<Utc>,
        deps: HookDeps,
    ) -> Self {
        Self {
            log: TeardownLogHook::new((&deps.options).into()),
            callbacks: TeardownCallbackHook::new(deps.callbacks.clone()),

            #[cfg(feature = "progress")]
            progress: deps.options.progress_support().then(||
                TeardownProgressHook::new(deps.progress_bars, transition_map, started_at, deps.options.timeout)
            ),
        }
    }
    pub(crate) async fn drop_in(self, duration: Duration) {
        tokio::time::sleep(duration).await;
    }
}
impl TeardownHook for HookDispatcher {
    fn on_task_unregistered(
        &self,
        transition_map: &TransitioningTaskMap,
        task_name: &'static str,
    ) {
        #[cfg(feature = "progress")]
        if let Some(progress) = self.progress.as_ref() {
            progress.on_task_unregistered(transition_map, task_name);
        }

        self.log.on_task_unregistered(transition_map, task_name);
    }

    fn on_finished(&self, stats: &TeardownStats) {
        // handle progress first so that log messages appear under the leftover timeout/task bars
        #[cfg(feature = "progress")]
        if let Some(progress) = self.progress.as_ref() {
            progress.on_finished(stats);
        }

        self.log.on_finished(stats);

        self.callbacks.on_finished(stats);
    }

    fn on_timeout(&self, stats: &TeardownTimeoutStats) {
        // handle progress first so that log messages appear under the leftover timeout/task bars
        #[cfg(feature = "progress")]
        if let Some(progress) = self.progress.as_ref() {
            progress.on_timeout(stats);
        }

        self.log.on_timeout(stats);

        self.callbacks.on_timeout(stats);
    }
}

// !- Logging hook

#[allow(clippy::struct_field_names)]
#[derive(Debug, Copy, Clone)]
pub(crate) struct TeardownLogHookOpts {
    tasks_level: Option<Level>,
    stats_teardown_level: Option<Level>,
    stats_timeout_level: Option<Level>,
}
impl From<&ShutdownConfig> for TeardownLogHookOpts {
    fn from(config: &ShutdownConfig) -> Self {
        Self {
            tasks_level: config.log_teardown_remaining.then_some(config.log_teardown_remaining_level),
            stats_teardown_level: config.log_teardown_stats.then_some(config.log_teardown_stats_level),
            stats_timeout_level: config.log_timeout_stats.then_some(config.log_timeout_stats_level),
        }
    }
}
pub(crate) struct TeardownLogHook {
    opts: TeardownLogHookOpts,
}
impl TeardownLogHook {
    pub(crate) fn new(opts: TeardownLogHookOpts) -> Self {
        Self { opts }
    }
}
impl TeardownHook for TeardownLogHook {
    fn on_task_unregistered(
        &self,
        transition_map: &TransitioningTaskMap,
        task_name: &'static str,
    ) {
        //let counts = transition_map.0.get(task_name).unwrap();
        tracing::debug!("Task instance finished: {task_name}");

        if let Some(level) = self.opts.tasks_level && transition_map.has_active_tasks() {
            let mut rem = transition_map.as_list().into_active_filtered();
            rem.sort_tasks();
            let name_sty = Style::new().bold();
            let count_sty = Style::new().dim();
            let rem = rem.0.into_iter().map(|t| format!(
                "{} [{}]",
                name_sty.apply_to(t.task_name()),
                count_sty.apply_to(format!("{}x", t.remaining_count())),
            )).collect::<Vec<_>>().join(", ");

            // tracing doesn't support dynamic levels, this preserves performance
            match level {
                Level::INFO => tracing::info!("Remaining tasks: {rem}"),
                Level::DEBUG => tracing::debug!("Remaining tasks: {rem}"),
                Level::TRACE => tracing::trace!("Remaining tasks: {rem}"),
                Level::WARN => tracing::warn!("Remaining tasks: {rem}"),
                Level::ERROR => tracing::error!("Remaining tasks: {rem}"),
            }
        }
    }

    fn on_finished(&self, stats: &TeardownStats) {
        tracing::info!("All tasks gracefully torn down.");
        tracing::trace!("Teardown completed in {} ms", stats.duration().as_millis());

        let task_count = stats.total_tasks();
        tracing::info!(
            started_at=?stats.started_at(),
            finished_at=?stats.finished_at(),
            elapsed_ms=stats.duration().as_millis(),
            task_count,
            "Teardown finished ({task_count} tasks stopped gracefully in {})",
            stats.duration_text(),
        );
        if let Some(level) = self.opts.stats_teardown_level {
            match level {
                Level::INFO => tracing::info!("\n{}", stats.report_text()),
                Level::DEBUG => tracing::debug!("\n{}", stats.report_text()),
                Level::TRACE => tracing::trace!("\n{}", stats.report_text()),
                Level::WARN => tracing::warn!("\n{}", stats.report_text()),
                Level::ERROR => tracing::error!("\n{}", stats.report_text()),
            }
        }
        //tracing::trace!("Full teardown stats:\n{stats:#?}");
    }

    fn on_timeout(&self, stats: &TeardownTimeoutStats) {
        let timed_out_task_count = stats.tasks().active_task_total();
        let timed_out_instance_count = stats.tasks().instances_remaining_total();

        tracing::error!("Teardown timed out - {timed_out_instance_count} instances from {timed_out_task_count} remaining tasks were timed out after {}s", stats.timeout().as_secs());

        if let Some(level) = self.opts.stats_timeout_level {
            match level {
                Level::INFO => tracing::info!("\n{}", stats.report_text()),
                Level::DEBUG => tracing::debug!("\n{}", stats.report_text()),
                Level::TRACE => tracing::trace!("\n{}", stats.report_text()),
                Level::WARN => tracing::warn!("\n{}", stats.report_text()),
                Level::ERROR => tracing::error!("\n{}", stats.report_text()),
            }
        }
        //tracing::trace!("Full teardown timeout stats:\n{stats:#?}");
    }
}

// !- User callback hook

pub(crate) type TeardownCallback = dyn Fn(&TeardownStats) + Sync + Send + 'static;
pub(crate) type TimeoutCallback = dyn Fn(&TeardownTimeoutStats) + Sync + Send + 'static;

#[derive(Clone, Default)]
pub(crate) struct TeardownCallbacks {
    pub(crate) on_teardown: Option<Arc<TeardownCallback>>,
    pub(crate) on_timeout: Option<Arc<TimeoutCallback>>,
}
impl fmt::Debug for TeardownCallbacks {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TeardownCallbacks")
            .field("on_teardown", if self.on_teardown.is_some() {
                &"Some(Fn(TeardownStats))"
            } else {
                &"None"
            })
            .field("on_timeout", if self.on_teardown.is_some() {
                &"Some(Fn(TeardownTimeoutStats))"
            } else {
                &"None"
            })
            .finish()
    }
}
pub(crate) struct TeardownCallbackHook {
    callbacks: TeardownCallbacks,
}
impl TeardownCallbackHook {
    pub(crate) fn new(callbacks: TeardownCallbacks) -> Self {
        Self { callbacks }
    }
}
impl TeardownHook for TeardownCallbackHook {
    fn on_finished(&self, stats: &TeardownStats) {
        if let Some(f) = &self.callbacks.on_teardown {
            f(stats);
        }
    }
    fn on_timeout(&self, stats: &TeardownTimeoutStats) {
        if let Some(f) = &self.callbacks.on_timeout {
            f(stats);
        }
    }
}
