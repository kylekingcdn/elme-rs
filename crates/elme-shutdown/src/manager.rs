#![allow(clippy::missing_panics_doc)]

use crate::{
    command::{Command, CommandResult, ReloadResult, StopCommand, StopResult},
    config::ShutdownConfig,
    signal::SignalHandler,
    state::{
        InformStartingError, InformStartedError,
        LifecycleStage, LockingSharedState, RegisterError,
        RunState, SharedState,
    },
    task::{
        InstanceCount,
        handle::TaskHandle,
        list::TrackedTaskList,
    },
    teardown::stats::{TeardownStats, TeardownTimeoutStats},
};

use std::sync::{Arc, Mutex};

// !- Manager

#[derive(Clone)]
pub struct ShutdownManager {
    shared: LockingSharedState,
}

impl ShutdownManager {
    #[must_use]
    fn new(
        options: ShutdownConfig,
        #[cfg(feature = "progress")]
        mp: indicatif::MultiProgress,
    ) -> Self {
        let state = SharedState::new(
            options,
            #[cfg(feature = "progress")]
            mp,
        );
        Self {
            shared: Arc::new(Mutex::new(state)),
        }
    }
    #[must_use]
    pub fn init(options: ShutdownConfig) -> Self {
        let manager = Self::new(
            options,
            #[cfg(feature = "progress")]
            indicatif::MultiProgress::default(),
        );
        if options.handle_signals() {
            SignalHandler::new_initialized(manager.shared.clone());
        }
        manager
    }
    #[cfg(feature = "progress")]
    #[cfg_attr(docsrs, doc(cfg(feature = "progress")))]
    #[must_use]
    pub fn init_with_multi_progress(
        options: ShutdownConfig,
        multi_progress: indicatif::MultiProgress,
    ) -> Self {
        let manager = Self::new(options, multi_progress);
        if options.handle_signals() {
            SignalHandler::new_initialized(manager.shared.clone());
        }
        manager
    }

    /// Options used to initialize `ShutdownManager`
    #[must_use]
    pub fn options(&self) -> ShutdownConfig {
        self.shared.lock().unwrap().options()
    }

    #[cfg(feature = "progress")]
    #[cfg_attr(docsrs, doc(cfg(feature = "progress")))]
    #[must_use]
    pub fn progress_bars(&self) -> indicatif::MultiProgress {
        self.shared.lock().unwrap().hook_deps().progress_bars.clone()
    }
    #[cfg(all(
        feature = "progress",
        feature = "progress-writer"
    ))]
    #[cfg_attr(docsrs, doc(cfg(all(
        feature = "progress",
        feature = "progress-writer"
    ))))]
    #[must_use]
    pub fn progress_writer(&self) -> crate::progress::writer::ProgressWriter {
        let mp = self.shared.lock().unwrap().hook_deps().progress_bars.clone();
        crate::progress::writer::ProgressWriter::new(mp)
    }

    #[must_use]
    pub fn issued_command(&self) -> Option<Command> {
        self.shared.lock().unwrap().issued_command()
    }
    #[must_use]
    pub fn pending_command(&self) -> Option<Command> {
        self.shared.lock().unwrap().pending_command()
    }

    /// Current stage of app lifecycle
    /// s
    #[must_use]
    pub fn lifecycle_stage(&self) -> LifecycleStage {
        self.shared.lock().unwrap().lifecycle_stage()
    }
    // current state of app
    #[must_use]
    pub fn run_state(&self) -> RunState {
        self.shared.lock().unwrap().run_state()
    }

    /// returns true until a stop command has been issued
    #[must_use]
    pub fn app_should_start(&self) -> bool {
        self.shared.lock().unwrap().app_should_start()
    }
    pub fn inform_starting(&self) -> Result<(), InformStartingError> {
        SharedState::inform_starting(&self.shared)
    }
    pub fn inform_started(&self) -> Result<(), InformStartedError> {
        SharedState::inform_started(&self.shared)
    }

    #[must_use]
    pub fn task_list(&self) -> TrackedTaskList {
        let mut task_list = self.shared.lock().unwrap().task_registry().as_task_list().into_active_filtered();
        task_list.sort_tasks();
        task_list
    }
    #[must_use]
    pub fn active_task_count(&self) -> usize {
        self.shared.lock().unwrap().task_registry().active_task_count()
    }
    #[must_use]
    pub fn has_active_tasks(&self) -> bool {
        !self.shared.lock().unwrap().task_registry().has_active_tasks()
    }

    #[must_use]
    pub fn total_instance_count(&self) -> InstanceCount {
        self.shared.lock().unwrap().task_registry().total_instance_count()
    }
    #[must_use]
    pub fn task_instance_count(&self, task_name: &'static str) -> InstanceCount {
        self.shared.lock().unwrap().task_registry().task_instance_count(task_name)
    }

    pub fn register_task(&self, task_name: &'static str) -> Result<TaskHandle, RegisterError> {
        SharedState::register_task(&self.shared, task_name)
    }

    #[allow(clippy::missing_panics_doc)]
    pub fn wait_for_teardown_start(&self) -> impl Future<Output=()> {
        let token = self.shared.lock().unwrap().teardown_start_token();
        token.cancelled_owned()
    }
    #[allow(clippy::missing_panics_doc)]
    pub fn wait_for_teardown_done(&self) -> impl Future<Output=()> {
        let token = self.shared.lock().unwrap().teardown_done_token();
        token.cancelled_owned()
    }

    #[must_use]
    pub fn issue_command(&self, command: Command) -> CommandResult {
        match command {
            Command::Reload => self.reload().into(),
            Command::Stop(stop_cmd)  => self.stop(stop_cmd.exit_code).into(),
        }
    }
    #[must_use]
    pub fn reload(&self) -> ReloadResult {
        tracing::info!("Received request to reload");
        let res = self.shared.lock().unwrap().reload();
        match &res {
            ReloadResult::Issued => {
                tracing::info!("Reload issued successfully. Starting teardown procedures");
                SharedState::start_teardown(&self.shared);
                //self.teardown();
            },
            ReloadResult::IssuedPending  => {
                tracing::info!("Reload is pending and will be issued upon finishing startup");
            },
            ReloadResult::AlreadyIssued  => {
                tracing::info!("Ignoring ShutdownManager::reload() call - was already issued");
            },
            ReloadResult::Stopping(stop_cmd) => {
                tracing::info!("Ignoring ShutdownManager::reload() call - stop has been called: {stop_cmd}");
            },
        }
        res
    }
    #[must_use]
    pub fn stop(&self, exit_code: i32) -> StopResult {
        tracing::info!(exit_code, "Received request to stop (exit code: {exit_code})");
        let res = self.shared.lock().unwrap().stop(exit_code);
        match &res {
            StopResult::Issued(_) => {
                tracing::info!("Stop issued successfully. Starting teardown procedures");
                SharedState::start_teardown(&self.shared);
                //self.teardown();
            },
            StopResult::IssuedPending(_) => {
                tracing::info!("Stop is pending and will be issued upon finishing startup");
            },
            StopResult::AlreadyIssued(StopCommand { exit_code }) => {
                tracing::info!("Already stopping (exit code: {exit_code}) - request ignored.");
            },
        }
        res
    }

    /// An optional callback/closure fn that is called once teardown completes.
    ///
    /// The provided fn will only be executed if **all tasks are stopped gracefully
    /// before the timeout is reached**.
    ///
    /// Supports builder-style method chaining (`mut` is not required).
    ///
    /// Replaces any fn's provided by prior invocations. To unset the callback,
    /// use [`unset_on_teardown()`](Self::unset_on_teardown).
    ///
    /// # Parameters
    ///
    /// The provided fn receives a single parameter: [`&TeardownStats`](TeardownStats).
    ///
    /// # Usage
    ///
    /// The intended use-case is for alternative handling or reporting of teardown stats.
    ///
    /// This should **not** be used to handle cleanup / shutdown procedures.
    ///
    /// # Behavior
    ///
    /// If the teardown was triggered with a stop command, the application
    /// will terminate ~immmediatrly after the provided fn returns.
    ///
    /// Otherwise - for a reload command - app startup should begin ~immediately after.
    ///
    /// # Related
    ///
    /// - [`unset_on_teardown()`](Self::unset_on_teardown)
    /// - [`ShutdownConfig::log_teardown_stats`]
    /// - [`on_timeout()`](Self::on_timeout)
    #[allow(clippy::must_use_candidate)]
    pub fn on_teardown(&self, f: impl Fn(&TeardownStats) + Send + Sync + 'static) {
        self.shared.lock().unwrap().on_teardown(f);
    }

    /// Removes the callback assigned via [`on_teardown()`](Self::on_teardown)
    ///
    /// Supports builder-style method chaining (`mut` is not required).
    #[allow(clippy::must_use_candidate)]
    pub fn unset_on_teardown(&self) {
        self.shared.lock().unwrap().unset_on_teardown();
    }

    /// An optional callback/closure fn that is called if the teardown timeout is reached.
    ///
    /// The provided fn will only be executed if **the teardown timeout is reached before all
    /// tasks have gracefully stopped**.
    ///
    /// Supports builder-style method chaining (`mut` is not required).
    ///
    /// Replaces any fn's provided by prior invocations. To unset the callback,
    /// use [`unset_on_timeout()`](Self::unset_on_teardown).
    ///
    /// # Parameters
    ///
    /// The provided fn receives a single parameter: [`&TeardownTimeoutStats`](TeardownTimeoutStats).
    ///
    /// # Usage
    ///
    /// The intended use-case is for alternative handling or reporting of timeout stats.
    ///
    /// This should **not** be used to handle recovery attempts / cleanup / shutdown procedures.
    ///
    /// # Behavior
    ///
    /// The application will **always** terminate ~immediately after this fn is called,
    /// regardless of the issued command.
    ///
    /// # Related
    ///
    /// - [`unset_on_timeout()`](Self::unset_on_timeout)
    /// - [`ShutdownConfig::log_timeout_stats`]
    /// - [`on_teardown()`](Self::on_teardown)
    #[allow(clippy::must_use_candidate)]
    pub fn on_timeout(&self, f: impl Fn(&TeardownTimeoutStats) + Send + Sync + 'static) -> &Self {
        self.shared.lock().unwrap().on_timeout(f);
        self
    }

    /// Removes the callback assigned via [`on_timeout()`](Self::on_timeout)
    ///
    /// Supports builder-style method chaining (`mut` is not required).
    #[allow(clippy::must_use_candidate)]
    pub fn unset_on_timeout(&self) -> &Self {
        self.shared.lock().unwrap().unset_on_timeout();
        self
    }
}

impl From<LockingSharedState> for ShutdownManager {
    fn from(shared: LockingSharedState) -> Self {
        Self {
            shared
        }
    }
}
impl From<&LockingSharedState> for ShutdownManager {
    fn from(shared: &LockingSharedState) -> Self {
        shared.to_owned().into()
    }
}
