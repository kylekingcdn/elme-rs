use crate::{
    command::{Command, CommandResult, ReloadResult, StopCommand, StopResult},
    config::ShutdownConfig,
    signal::SignalHandler,
    state::{
        InformStartingError, InformStartedError,
        LifecycleStage, LockingSharedState,
        RunState, SharedState,
    },
    task::{
        InstanceCount,
        handle::TaskHandle,
        list::TrackedTaskList,
        RegisterError,
    },
    teardown::stats::{TeardownStats, TeardownTimeoutStats},
};

use std::sync::{Arc, Mutex, MutexGuard};

// !- Manager

/// Primary management interface for `elme-shutdown`.
///
/// # Sharing manager access
///
/// `ShutdownManager` can be cloned for thread-safe, shared access.
/// Cloning is cheap as all internal data is wrapped in a single `Arc`.
///
/// ## Access via `TaskHandle`
///
/// You can also access the `ShutdownManager` instance from the [`manager()`](TaskHandle::manager) method provided by a [`TaskHandle`].
///
/// Resolving a manager from a `TaskHandle` makes it simple to perform actions from workers (such as triggering a shutdown), without polluting your entite call tree with `ShutdownManager` params.
///
/// There is no additional overhead incurred by accessing a manager using a task handle. Internally, it is identical to calling `clone()` on a manager.
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
        // init shared state
        let state = SharedState::new(
            options,
            #[cfg(feature = "progress")]
            mp,
        );
        let shared = Arc::new(Mutex::new(state));

        // register global signal handler
        if options.handle_signals() {
            SignalHandler::new_initialized(shared.clone());
        }

        Self {
            shared,
        }
    }

    /// Initializes `elme-shutdown` with the provided options.
    ///
    /// This **must not** be called more than once throughout the lifetime of your application.
    ///
    /// If [`ShutdownConfig::handle_signals`](crate::ShutdownConfig::handle_signals) is enabled, this will additionally spawn the signal monitor.
    ///
    /// # Thread-safety
    ///
    /// All inner `ShutdownManager` data is wrapped in an `Arc`, and thus can be cheaply
    /// cloned and used in async contexts.
    #[must_use]
    pub fn init(options: ShutdownConfig) -> Self {
        Self::new(
            options,
            #[cfg(feature = "progress")]
            indicatif::MultiProgress::default(),
        )
    }

    /// Initializes `elme-shutdown` with options and a user-provided [`indicatif::MultiProgress`].
    ///
    /// The standard [`init()`](Self::init) fn will construct a new `MultiProgress` instance (if the `progress` feature is enabled).
    ///
    /// If your app is already serving progress bars via [`indicatif`], you can provide
    /// your existing `MultiProgress` to (hopefully) retain dual-support.
    ///
    /// ---
    ///
    /// # Using `tracing-subscriber` `Writer`'s
    ///
    /// Multiple [`tracing-subscriber`](::tracing_subscriber) `Writer` layers made for use with [`indicatif`](::indicatif) should not
    /// be installed simultaneously, this includes:
    /// - `elme_shutdown::ProgressWriter` (gated by `progress-writer`)
    /// - `tracing-indicatif::IndicatifWriter`
    ///
    /// Only a single `indicatif` writer should be used at a given time.
    ///
    /// ---
    ///
    /// # Inner `MultiProgress` access
    ///
    /// The underlying [`MultiProgress`](indicatif::MultiProgress) can be accessed via [`progress_bars()`](Self::progress_bars).
    ///
    /// This is supported for both the [`init()`](Self::init) and [`init_with_multi_progress()`](Self::init_with_multi_progress) fns.
    #[cfg(feature = "progress")]
    #[cfg_attr(docsrs, doc(cfg(feature = "progress")))]
    #[must_use]
    pub fn init_with_multi_progress(
        options: ShutdownConfig,
        multi_progress: indicatif::MultiProgress,
    ) -> Self {
        Self::new(options, multi_progress)
    }

    /// Options used to initialize `ShutdownManager`
    #[must_use]
    pub fn options(&self) -> ShutdownConfig {
        self.shared_lock().options()
    }

    /// Returns the underlying [`indicatif::MultiProgress`]
    ///
    /// Can be used to add support for additional progress bars while avoiding output conflicts.
    #[cfg(feature = "progress")]
    #[cfg_attr(docsrs, doc(cfg(feature = "progress")))]
    #[must_use]
    pub fn progress_bars(&self) -> indicatif::MultiProgress {
        self.shared_lock().hook_deps().progress_bars.clone()
    }

    /// Creates a new [`ProgressWriter`](crate::progress::writer::ProgressWriter)
    /// for use with `tracing-subscriber`.
    ///
    /// This is identical to:
    ///
    /// ```
    /// # use elme_shutdown::{ProgressWriter, ShutdownConfig, ShutdownManager};
    /// #
    /// # let config = ShutdownConfig::builder().handle_signals(false).build();
    /// # let shutdown_manager = ShutdownManager::init(config);
    /// let writer = ProgressWriter::new(shutdown_manager.progress_bars());
    /// ```
    ///
    /// See the [`ProgressWriter`](crate::progress::writer::ProgressWriter) docs for more info.
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
        let mp = self.progress_bars();
        crate::progress::writer::ProgressWriter::new(mp)
    }

    /// used to centralize `missing_panic_docs` warnings without blanket allow
    pub(crate) fn shared_lock(&self) -> MutexGuard<'_, SharedState> {
        self.shared.lock().unwrap()
    }

    // !- State management

    /// Whether or not the application should start & run.
    ///
    /// This will always return true, so long as [`issued_command`](Self::issued_command) is not `Stop`.
    ///
    /// Once this does return `false`, it is guaranteed to not return `true` for the remainder
    /// of the application's lifetime - as no other commands can be issued after `Stop`.
    #[must_use]
    pub fn app_should_start(&self) -> bool {
        self.shared_lock().app_should_start()
    }

    /// Instructs the `ShutdownManager` that startup procedures are underway.
    ///
    /// # Usage
    ///
    #[doc = include_str!("../doc/main_fn.md")]
    ///
    /// # Guarantees
    ///
    /// If a command (`Stop`/`Reload`) is given during startup, it will **not** be issued until
    /// startup is completed. The command will be stored as the [`pending_command`](Self::pending_command), and will not be processed until `inform_started` is called.
    ///
    /// This guarantee is in place to help avoid state de-sync / mid-startup uncertainty.
    ///
    /// This guarantee further asserts that teardown will never begin while startup is in progress.
    ///
    /// This works in harmony with the task registration guarantees:
    /// - `register_task()` will always fail during teardown
    /// - `register_task()` never fail outside teardown). It can therefore be deduced that registration will never fail during startup.
    ///
    /// Because of this added safety-net, it's highly recommended that any workers have task registration peformed during startup. This implementation sees `TaskHandle`s stored as member fields of the worker, as opposed to passing a `TaskHandle` into the worker's `run()` method.
    /// <!-- TODO: does this tenet require that worker run() fns take ownership of Self? -->
    ///
    /// # Errors
    ///
    /// Returns an [`InformStartingError`] if a transition to `Starting` is not currently valid.
    ///
    /// The restrictions in place, along with the associated error variants, are:
    ///
    /// - If the application is already in the startup stage (`inform_starting()` callled > 1x)
    ///   - Returns [`InformStartingError::AlreadyStarting`]
    ///   - This does not include the implicit state encountered immediately at launch. No custom
    ///     logic is required for first launch.
    ///
    /// - If the current state is not eligible for transition to `Starting`
    ///   - Returns [`InformStartingError::InvalidState`]
    ///   - The only valid transitions are:
    ///     1. The application is in its first start-up sequence.
    ///        - This is the first state change that must occur at launch. No other transitions are
    ///          supported
    ///     2. From teardown
    ///        - Teardown needs to have completed successfully
    ///
    /// - If teardown has finished successfully, but the command is not `Reload`
    ///   - Returns [`InformStartingError::SubsequentStartNonReload`]
    ///     - Startup should never be re-attempted after `Stop` is issued.
    ///     - The [`pending_command`](Self::pending_command) is not considered here.
    pub fn inform_starting(&self) -> Result<(), InformStartingError> {
        SharedState::inform_starting(&self.shared)
    }

    /// Instructs the `ShutdownManager` that startup has finished and the application is ready.
    ///
    /// # Usage
    ///
    #[doc = include_str!("../doc/main_fn.md")]
    ///
    /// # Errors
    ///
    /// Returns an [`InformStartedError`] if a transition to `Running` is not currently valid.
    ///
    /// The restrictions in place, along with the associated error variants, are:
    /// - If the application is not currently in the `Starting` stage (via `inform_starting()`)
    ///   - Returns [`InformStartedError::NotStarting`]
    ///   - `inform_started()` must be called after [`inform_starting()`](Self::inform_starting)
    pub fn inform_started(&self) -> Result<(), InformStartedError> {
        SharedState::inform_started(&self.shared)
    }

    #[must_use]
    pub fn stop(&self, exit_code: u8) -> StopResult {
        tracing::info!(exit_code, "Received request to stop (exit code: {exit_code})");
        let res = self.shared_lock().stop(exit_code);
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
    #[must_use]
    pub fn reload(&self) -> ReloadResult {
        tracing::info!("Received request to reload");
        let res = self.shared_lock().reload();
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
    pub fn issue_command(&self, command: Command) -> CommandResult {
        match command {
            Command::Reload => self.reload().into(),
            Command::Stop(stop_cmd)  => self.stop(stop_cmd.exit_code).into(),
        }
    }

    /// If a stop command has been issued, returns the inner exit code - otherwise `None`
    #[must_use]
    pub fn exit_code(&self) -> Option<u8> {
        if let Some(Command::Stop(cmd)) = self.issued_command() {
            Some(cmd.exit_code)
        } else {
            None
        }
    }
    /// The currently issued [`Command`].
    ///
    /// # Issued vs. Pending
    ///
    /// If a command is issued while the application is in it's startup stage, it will be
    /// stored to [`pending_command`](Self::pending_command) until startup completes ([`inform_started`](Self::inform_started) is called).
    ///
    /// Once startup completes, the pending command will replace the issued command (if any),
    /// and teardown will begin.
    #[must_use]
    pub fn issued_command(&self) -> Option<Command> {
        self.shared_lock().issued_command()
    }

    /// The pending [`Command`].
    ///
    /// # Issued vs. Pending
    ///
    /// If a command is issued while the application is in it's startup stage, it will be
    /// stored to [`pending_command`](Self::pending_command) until startup completes ([`inform_started`](Self::inform_started) is called).
    ///
    /// Once startup completes, the pending command will replace the issued command (if any),
    /// and teardown will begin.
    #[must_use]
    pub fn pending_command(&self) -> Option<Command> {
        self.shared_lock().pending_command()
    }

    /// Current stage of app lifecycle
    #[must_use]
    pub fn lifecycle_stage(&self) -> LifecycleStage {
        self.shared_lock().lifecycle_stage()
    }
    #[must_use]
    pub fn run_state(&self) -> RunState {
        self.shared_lock().run_state()
    }

    // !- Task management

    pub fn register_task(&self, task_name: &'static str) -> Result<TaskHandle, RegisterError> {
        SharedState::register_task(&self.shared, task_name)
    }

    #[must_use]
    pub fn task_list(&self) -> TrackedTaskList {
        let mut task_list = self.shared_lock().task_registry().as_task_list().into_active_filtered();
        task_list.sort_tasks();
        task_list
    }
    #[must_use]
    pub fn active_task_count(&self) -> usize {
        self.shared_lock().task_registry().active_task_count()
    }
    #[must_use]
    pub fn has_active_tasks(&self) -> bool {
        !self.shared_lock().task_registry().has_active_tasks()
    }

    #[must_use]
    pub fn total_instance_count(&self) -> InstanceCount {
        self.shared_lock().task_registry().total_instance_count()
    }
    #[must_use]
    pub fn task_instance_count(&self, task_name: &'static str) -> InstanceCount {
        self.shared_lock().task_registry().task_instance_count(task_name)
    }

    // !- Teardown futures

    pub fn wait_for_teardown_start(&self) -> impl Future<Output=()> {
        let token = self.shared_lock().teardown_start_token();
        token.cancelled_owned()
    }
    pub fn wait_for_teardown_done(&self) -> impl Future<Output=()> {
        let token = self.shared_lock().teardown_done_token();
        token.cancelled_owned()
    }

    // !- Teardown callbacks

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
    /// will terminate ~immediately after the provided fn returns.
    ///
    /// Otherwise - for a reload command - app startup should begin ~immediately after.
    ///
    /// # Related
    ///
    /// - [`unset_on_teardown()`](Self::unset_on_teardown)
    /// - [`ShutdownConfig::log_teardown_stats`]
    /// - [`on_timeout()`](Self::on_timeout)
    pub fn on_teardown(&self, f: impl Fn(&TeardownStats) + Send + Sync + 'static) -> &Self {
        self.shared_lock().on_teardown(f);
        self
    }

    /// Removes the callback assigned via [`on_teardown()`](Self::on_teardown)
    ///
    /// Supports builder-style method chaining (`mut` is not required).
    #[allow(clippy::must_use_candidate)]
    pub fn unset_on_teardown(&self) -> &Self {
        self.shared_lock().unset_on_teardown();
        self
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
    pub fn on_timeout(&self, f: impl Fn(&TeardownTimeoutStats) + Send + Sync + 'static) -> &Self {
        self.shared_lock().on_timeout(f);
        self
    }

    /// Removes the callback assigned via [`on_timeout()`](Self::on_timeout)
    ///
    /// Supports builder-style method chaining (`mut` is not required).
    #[allow(clippy::must_use_candidate)]
    pub fn unset_on_timeout(&self) -> &Self {
        self.shared_lock().unset_on_timeout();
        self
    }
}

impl Default for ShutdownManager {
    fn default() -> Self {
        Self::init(ShutdownConfig::default())
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
