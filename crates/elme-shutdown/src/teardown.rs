pub mod stats;

use self::stats::{CommonStats, TeardownStats, TeardownTimeoutStats};
use crate::{
    config::ShutdownConfig,
    task::{
        map::{TrackedTaskMap, TransitioningTaskMap},
        registry::RegistrationMessage,
    },
};
#[cfg(feature = "progress")]
use crate::progress::TeardownProgressHook;

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use tokio_util::sync::CancellationToken;

// !- Monitor

#[derive(Debug)]
pub(crate) struct TeardownMonitorParams {
    pub options: ShutdownConfig,
    pub initial_tasks: TrackedTaskMap,
    pub registration_rx: broadcast::Receiver<RegistrationMessage>,
    pub hook_deps: HookDeps,
}

pub(crate) struct TeardownMonitor {
    params: TeardownMonitorParams,
}
impl TeardownMonitor {
    // should not be used until after notified is issued
    pub fn new(params: TeardownMonitorParams) -> Self {
        Self {
            params,
        }
    }
    pub async fn start(self) -> TeardownResult {
        // token to be cancelled once all tasks have been gracefully stopped or on timeout
        let finished_token = CancellationToken::new();
        let (unregister_tx, unregister_rx) = mpsc::unbounded_channel();
        let timeout = self.params.options.timeout();

        let message_proxy = MessageProxy::new(finished_token.clone(), unregister_tx, timeout);
        let unregister_handler = UnregisterHandler::new(finished_token, unregister_rx, timeout, self.params.initial_tasks, self.params.hook_deps);

        // start message proxy
        let ((), result) = tokio::join!(
            message_proxy.start(self.params.registration_rx),
            unregister_handler.start(),
        );

        tracing::trace!("Got teardown monitor result: {result:#?}");

        result
    }
}

// !- Unregister handler

/// receives unregister or timeout messages
///
/// timeout handling is bundled in to allow for returning local state without
/// having to rely on arc wrapped mutexes
struct UnregisterHandler {
    finished_token: CancellationToken,
    rx: mpsc::UnboundedReceiver<UnregisterMessage>,
    timeout: Duration,

    initial_tasks: TrackedTaskMap,
    transition_map: TransitioningTaskMap,
    hook_deps: HookDeps,
}
impl UnregisterHandler {
    pub fn new(
        finished_token: CancellationToken,
        rx: mpsc::UnboundedReceiver<UnregisterMessage>,
        timeout: Duration,
        initial_tasks: TrackedTaskMap,
        hook_deps: HookDeps,
    ) -> Self {
        let transition_map = initial_tasks.clone().into();
        Self {
            finished_token,
            rx,
            timeout,
            initial_tasks,
            transition_map,
            hook_deps,
        }
    }
    pub async fn start(mut self) -> TeardownResult {
        let started_at = Utc::now();

        let hook_dispatcher = HookDispatcher::new(
            &self.transition_map,
            started_at,
            self.timeout,
            self.hook_deps.clone(),
        );

        while let Some(msg) = self.rx.recv().await {
            tracing::debug!("UnregisterHandler got message: {msg:?}");
            match msg {
                UnregisterMessage::Timeout => {
                    tracing::warn!("Unregister handler has timed out - wrapping up");
                    hook_dispatcher.on_timeout(
                        &self.transition_map,
                        started_at,
                        Utc::now(),
                        self.timeout,
                    );
                    break;
                }
                UnregisterMessage::Unregister(task_name) => {
                    if let Err(error) = self.transition_map.try_deduct_remaining(task_name) {
                        tracing::error!("failed to deduct task count: {error}");
                        panic!("failed to deduct task count: {error}");
                    }

                    hook_dispatcher.on_task_unregistered(&self.transition_map, task_name);

                    // !- FIXME: use a log hook
                    {
                        let counts = self.transition_map.0.get(task_name).unwrap();
                        tracing::info!("Task instance finished: {task_name}. Remaining {task_name} instances: {}", counts.as_remaining_fraction());
                        self.log_remaining_tasks();
                    }

                    if !self.transition_map.has_active_tasks() {
                        tracing::info!("All tasks have finished");
                        hook_dispatcher.on_finished(
                            &self.transition_map,
                            started_at,
                            Utc::now(),
                            self.timeout,
                        );
                        break;
                    }
                }
            }
        }
        let finished_at = Utc::now();
        self.finished_token.cancel();

        // keep hook dispatcher alive briefly
        tokio::spawn(async move {
            let _ = hook_dispatcher;
            tokio::time::sleep(Duration::from_secs(1)).await;
        });

        self.generate_results(started_at, finished_at)
    }

    fn generate_results(&self, started_at: DateTime<Utc>, finished_at: DateTime<Utc>) -> TeardownResult {
        let mut final_tasks = TrackedTaskMap::from(self.transition_map.clone().into_active_filtered().0.into_iter().map(|t| (t.0, t.1.remaining())).collect::<HashMap<_,_>>()).as_list();
        final_tasks.sort_tasks();

        let mut transition_list = self.transition_map.as_list();
        transition_list.sort_tasks();

        let mut initial_tasks_list = self.initial_tasks.as_list().into_active_filtered();
        initial_tasks_list.sort_tasks();

        let common_stats = CommonStats {
            tasks: transition_list,
            started_at,
            finished_at,
            timeout: self.timeout,
        };
        if final_tasks.has_active_tasks() {
            let timed_out_tasks = self.transition_map.as_list();
            let timed_out_task_count = timed_out_tasks.active_task_count();
            let timed_out_instance_count = timed_out_tasks.instances_remaining_total();

            let stats = TeardownTimeoutStats::from(common_stats);

            // -- debug / logging, can be moved
            tracing::error!("Teardown timed out - {timed_out_instance_count} instances from {timed_out_task_count} remaining tasks were timed out after {}s", self.timeout.as_secs());
            tracing::warn!("Timeout stats:");
            tracing::info!("\n{}", stats.stats_table());
            tracing::trace!("Full teardown timeout stats:\n{stats:#?}");

            Err(stats)
        } else {
            let stats = TeardownStats::from(common_stats);
            // -- debug / logging, can be moved
            tracing::info!("All tasks gracefully torn down.");
            tracing::info!("Teardown completed in {} ms", stats.duration().as_millis());
            tracing::info!("\n{}", stats.stats_table());
            tracing::trace!("Full teardown stats:\n{stats:#?}");

            Ok(stats)
        }
    }

    fn log_remaining_tasks(&self) {
        let mut rem = self.transition_map.as_list().into_active_filtered();
        rem.sort_tasks();

        let rem = rem.0.into_iter().map(|t| format!("\x1b[1m{}\x1b[0m [\x1b[2m{}x\x1b[0m]", t.task_name(), t.remaining_count())).collect::<Vec<_>>().join(", ");
        tracing::info!("Remaining tasks: {rem}");
    }
}

pub type TeardownResult = Result<TeardownStats, TeardownTimeoutStats>;

// !- Proxied message

/// Proxied message for [`RegistrationMessage`] with a timeout command message.
///
/// Instances of [`RegistrationMessage::Register`] are dropped, and instances of
/// [`RegistrationMessage::Unregister`] are mapped directly to
/// [`UnretgisterMessage::Unregister`].
///
/// The timeout message is used to notify [`MessageProxy`] that the graceful
/// task stop threshold has been crossed and should stop processing unregistered
/// tasks.
#[derive(Debug, Copy, Clone)]
enum UnregisterMessage {
    Unregister(&'static str),
    Timeout,
}

// !- Unregister proxy

/// Forwards [`RegistrationMessage::Unregister`] messages to [`UnregisterHandler`].
///
/// Also provides the teardown timeout implementation by sending an [`UnregisterMessage::Timeout`] message
/// within a [`tokio::select!`] branch containing a [`tokio::time::sleep`] call.
struct MessageProxy {
    finished_token: CancellationToken,
    tx: mpsc::UnboundedSender<UnregisterMessage>,
    timeout: Duration,
}
impl MessageProxy {
    pub fn new(
        finished_token: CancellationToken,
        tx: mpsc::UnboundedSender<UnregisterMessage>,
        timeout: Duration,
    ) -> Self {
        Self {
            finished_token,
            tx,
            timeout,
        }
    }
    pub async fn start(self, rx: broadcast::Receiver<RegistrationMessage>) {
        tracing::info!("Starting TeardownMonitor message proxy");

        tokio::select! {
            biased;
            () = self.finished_token.cancelled() => {
                tracing::info!("MessageProxy got finished notification from handler");
            }
            () = tokio::time::sleep(self.timeout) => {
                tracing::info!("MessageProxy timed out");
                if let Err(error) = self.tx.send(UnregisterMessage::Timeout) {
                    tracing::error!("Failed to send timeout message: {error}");
                }
            }
            () = Self::forward_messages(rx, self.tx.clone()) => {
                tracing::error!("MessageProxy forward_messages() terminated early?");
            }
        };
    }

    async fn forward_messages(
        mut rx: broadcast::Receiver<RegistrationMessage>,
        tx: mpsc::UnboundedSender<UnregisterMessage>,
    ) {
        while let Ok(msg) = rx.recv().await {
            match msg {
                RegistrationMessage::Register(task_name) => {
                    tracing::error!("MessageProxy ignoring register message for task: {task_name}");
                }
                RegistrationMessage::Unregister(task_name) => {
                    tracing::debug!("MessageProxy forwarding unregister message for task: {task_name}");
                    if let Err(error) = tx.send(UnregisterMessage::Unregister(task_name)) {
                        tracing::error!("Failed to forward unregister message: {error}");
                    }
                }
            }
        }
    }
}

// !- Hook dispatcher

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
#[derive(Debug, Clone, Default)]
pub(crate) struct HookDeps {
    #[cfg(feature = "progress")]
    pub progress_bars: indicatif::MultiProgress
}
impl HookDeps {
    pub fn new() -> Self {
        Self::default()
    }
    #[cfg(feature = "progress")]
    #[allow(clippy::needless_update)]
    pub fn with_progress(progress_bars: indicatif::MultiProgress) -> Self {
        Self {
            progress_bars,
            ..Default::default()
        }
    }
}

struct HookDispatcher {
    #[cfg(feature = "progress")]
    progress: TeardownProgressHook,
}
#[allow(unused, clippy::unused_self, clippy::needless_pass_by_value)]
impl HookDispatcher {
    pub fn new(
        transition_map: &TransitioningTaskMap,
        started_at: DateTime<Utc>,
        timeout: Duration,
        deps: HookDeps,
    ) -> Self {
        Self {
            #[cfg(feature = "progress")]
            progress: TeardownProgressHook::new(deps.progress_bars, transition_map, started_at, timeout),
        }
    }

    pub fn on_task_unregistered(
        &self,
        transition_map: &TransitioningTaskMap,
        task_name: &'static str,
    ) {
        #[cfg(feature = "progress")]
        self.progress.on_task_unregistered(transition_map, task_name);
    }

    pub fn on_finished(
        &self,
        transition_map: &TransitioningTaskMap,
        started_at: DateTime<Utc>,
        finished_at: DateTime<Utc>,
        timeout: Duration,
    ) {
        #[cfg(feature = "progress")]
        self.progress.on_finished(transition_map, started_at, finished_at, timeout);
    }

    pub fn on_timeout(
        &self,
        transition_map: &TransitioningTaskMap,
        started_at: DateTime<Utc>,
        timeout_at: DateTime<Utc>,
        timeout: Duration,
    ) {
        #[cfg(feature = "progress")]
        self.progress.on_timeout(transition_map, started_at, timeout_at, timeout);
    }
}
