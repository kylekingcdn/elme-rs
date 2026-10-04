use crate::{
    command::{Command, CommandResult, ReloadResult, StopCommand, StopResult},
    config::{ShutdownConfig, ShutdownConfigBuilder},
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
/// Resolving a manager from a `TaskHandle` makes it simple to perform actions from workers (such as triggering a shutdown), without polluting your entire call tree with `ShutdownManager` params.
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

    /// Creates a new [`ShutdownConfigBuilder`]
    ///
    /// Identical to both [`ShutdownConfig::builder()`](ShutdownConfig::builder)
    /// and [`ShutdownConfigBuilder::new()`](ShutdownConfigBuilder::new)
    ///
    /// Provided for the sole purpose of reducing one-off import clutter.
    #[must_use]
    pub fn config_builder() -> ShutdownConfigBuilder {
        ShutdownConfig::builder()
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
    /// ---
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
    /// # Guarantees
    ///
    /// **A teardown will never begin during startup.**
    /// - If a `Stop` or `Reload` command is issued during startup, it will be stored as the
    ///   [`pending_command`](Self::pending_command).
    /// - Pending commands are not processed until `inform_started` is called.
    ///
    /// This guarantee is in place to ensure startup procedures remain consistent, thus prevennting
    /// numerous potential application-state bugs.
    ///
    /// Furthermore, this guarantee works in direct harmony with those provided by
    /// [`register_task()`](Self::register_task):
    /// - **`register_task()` will always fail during teardown**
    /// - **`register_task()` will never fail outside of teardown**
    ///
    /// With that, an additional guarantee can be deduced:
    /// - **Task registration never fails during startup**
    ///
    /// Because of this added safety-net, it's recommended that all workers with a non-fluctuating
    /// number of instances be registered during startup (in the same fashion as the example below).
    /// - While this can certainly simplify lifecycle management, it is by no means a hard
    ///   requirement.
    /// - There may be cases where you want to lazy-load a worker as it may not be needed on
    ///   every run.
    /// <!-- TODO: does this tenet require that worker run() fns take ownership of Self? -->
    ///
    /// # Usage
    ///
    #[doc = include_str!("../doc/main_fn.md")]
    ///
    /// # Errors
    ///
    /// Returns an [`InformStartingError`] if a transition to `Starting` is not currently valid.
    ///
    /// The restrictions in place, along with the associated error variants, are:
    ///
    /// - If the application is already in the startup stage
    ///   - Returns [`InformStartingError::AlreadyStarting`]
    ///   - This does not include the implicit state encountered immediately at launch. No custom
    ///     logic is required for first launch.
    ///
    /// - If the current state is not eligible for transition to `Starting`
    ///   - Returns [`InformStartingError::InvalidState`]
    ///   - The only valid transitions are from the following states:
    ///     1. The application is in its first start-up sequence
    ///     2. The application has successfully finished teardown
    ///
    /// - If teardown has finished successfully, but the issued command is not `Reload`
    ///   - Returns [`InformStartingError::SubsequentStartNonReload`]
    ///   - Startup should never be re-attempted after `Stop` is issued (implied by non-`Reload`).
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

    /// Sends a `Stop` command
    ///
    /// # Parameters
    ///
    /// - `exit_code`
    ///   - The code that the process should exit with.
    ///   - For a standard no-issue exit, this is typically `0`.
    ///
    /// # Returns
    ///
    /// Returns a [`StopResult`].
    ///
    /// While not a conventional `Result` with `Ok`/`Err` states, a `StopResult` does represent
    /// varying degrees of "success". Despite this, a call to `stop` will always result in either
    /// immediate or queued teardown and exit.
    /// Therefore, handling is optional.
    ///
    /// - If either [`issued_command`] or [`pending_command`] contain `Stop`,
    ///   returns [`StopResult::AlreadyIssued`]
    /// - Otherwise,
    ///   - If currently in startup, saved to `pending_command` and returns [`StopResult::IssuedPending`]
    ///   - If running or tearing down, saved to `issued_command` and returns [`StopResult::Issued`]
    ///
    /// **Note**: Issuing stop will directly replace an issued or pending `Reload`, as long as the reload hasn't already entered startup.
    ///
    /// The inner [`StopCommand`] contained in each variant will be the previously issued `Stop`, if any, or the command provided.
    ///
    /// [`issued_command`]: Self::issued_command
    /// [`pending_command`]: Self::pending_command
    #[must_use]
    pub fn stop(&self, exit_code: u8) -> StopResult {
        tracing::info!(exit_code, "Received request to stop (exit code: {exit_code})");
        let res = self.shared_lock().stop(exit_code);
        // !- TODO: remove
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

    /// Sends a `Reload` command
    ///
    /// # Returns
    ///
    /// Returns a [`ReloadResult`].
    ///
    /// While not a conventional `Result` with `Ok`/`Err` states, a `ReloadResult` does represent
    /// varying degrees of "success".
    ///
    /// - If either [`issued_command`] or [`pending_command`] contain `Stop`,
    ///   returns [`ReloadResult::Stopping`] with the issued `Stop` command
    /// - Otherwise, if either [`issued_command`] or [`pending_command`] contain `Reload`, returns
    ///   [`ReloadResult::AlreadyIssued`]
    /// - Otherwise,
    ///   - If currently in startup, saved to `pending_command` and returns
    ///     [`ReloadResult::IssuedPending`]
    ///   - If running, saved to `issued_command` and returns [`ReloadResult::Issued`]
    ///   - **Note:** there is no case for tearing down as this would imply either `Stop` or
    ///     `Reload` has been issued, which is covered by the first 2 cases.
    ///
    /// [`issued_command`]: Self::issued_command
    /// [`pending_command`]: Self::pending_command
    #[must_use]
    pub fn reload(&self) -> ReloadResult {
        tracing::info!("Received request to reload");
        let res = self.shared_lock().reload();
        // !- TODO: remove
        match &res {
            ReloadResult::Issued => {
                tracing::info!("Reload issued successfully. Starting teardown procedures");
                SharedState::start_teardown(&self.shared);
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

    /// Helper that calls either [`stop()`](Self::stop) or [`reload()`](Self::reload) based on the provided [`Command`]
    ///
    /// # Returns
    ///
    /// Returns a [`CommandResult`], an enum containing a variant for the type returned by each respective dispatched fn.
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
    /// If a command is issued while the application is in its startup stage, it will be
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
    /// If a command is issued while the application is in its startup stage, it will be
    /// stored to [`pending_command`](Self::pending_command) until startup completes ([`inform_started`](Self::inform_started) is called).
    ///
    /// Once startup completes, the pending command will replace the issued command (if any),
    /// and teardown will begin.
    #[must_use]
    pub fn pending_command(&self) -> Option<Command> {
        self.shared_lock().pending_command()
    }

    /// Current stage of the application's lifecycle
    ///
    /// # Lifecycle management
    ///
    /// Handling of an application's lifecycle is done via:
    /// - [`inform_starting()`](Self::inform_starting)
    /// - [`inform_started()`](Self::inform_started)
    /// - [`stop()`](Self::stop)
    /// - [`reload()`](Self::reload)
    ///
    /// ---
    ///
    /// See the [`LifecycleStage`] docs for more information.
    #[must_use]
    pub fn lifecycle_stage(&self) -> LifecycleStage {
        self.shared_lock().lifecycle_stage()
    }

    /// The application's current `RunState`
    ///
    /// ---
    ///
    /// See the [`RunState`] docs for more information.
    #[must_use]
    pub fn run_state(&self) -> RunState {
        self.shared_lock().run_state()
    }

    // !- Task management

    /// Creates a new [`TaskHandle`] for a task with the provided name
    ///
    /// # Errors
    ///
    /// Returns a [`RegisterError`] if task registration fails.
    ///
    /// Task registration only fails if the attempt occurs during teardown.
    ///
    /// # Guarantees
    ///
    /// - Task registration ***never fails*** during `Startup` or `Running` stages.
    /// - Task registration ***always fails*** during the `Teardown` stage.
    ///
    /// While task registration doesn't fail in the `Running` stage, it's entirely possible that
    /// teardown is started concurrently, resulting in a failed registration, even if the state
    /// was checked immediately prior.
    ///
    /// # Usage
    ///
    /// Workers that exist across the lifetime of your application should be built (and registered)
    /// during startup. It therefore makes the most sense to store handles as member fields,
    /// passing them (as owned) into the worker's constructor.
    /// The reason for this is described in the [Guarantees](#guarantees) section.
    ///
    /// The [No error handling example](#no-error-handling) shows how to initialize lifelong workers
    /// without error handling.
    ///
    /// # Purpose
    ///
    /// Task handles are used internally to keep track of which tasks (or more accurately, how many
    /// instances of a task) are still running. This is a critical aspect of graceful shutdown, as
    /// application exit will be delayed until all instance counters drop to 0.
    ///
    /// Registering a task (or cloning a handle) will increment the instance count associated with
    /// the task name).
    ///
    /// Once a `TaskHandle` goes out of scope (is dropped), the counter is decremented.
    ///
    /// Therefore, for graceful shutdown to work correctly, it's critical that all `TaskHandle`s
    /// are provided to workers as owned and not borrowed or cloned.
    /// (See the [Initializing concurrent workers](#initializing-concurrent-workers) example below
    /// for handling this scenario)
    ///
    /// # Examples
    ///
    /// ## Outside startup
    ///
    /// Here, we register a task outside startup, where registration can fail.
    ///
    /// The worker is created from another, pre-existing worker (that has its own task handle).
    ///
    /// ```
    /// # #[tokio::main]
    /// # async fn main() {
    #[doc = include_str!("../doc/elme_proxy.rs")]
    /// use elme::shutdown::TaskHandle;
    ///
    /// # let shutdown_mgr = elme_shutdown::ShutdownManager::default();
    /// # pub struct Worker { task_handle: TaskHandle };
    /// # impl Worker {
    /// # pub fn new(task_handle: TaskHandle) -> Self { Self { task_handle }}
    /// # async fn run(self) {}
    /// # async fn run_parent(self) {
    /// // worker only gets built (and runs) when registration succeeds
    /// if let Ok(task_handle) = self.task_handle.register_task("Child worker") {
    ///     let worker = Worker::new(task_handle);
    ///     tokio::spawn(async move { worker.run().await; });
    /// }
    /// # }}
    /// # }
    /// ```
    ///
    /// ## No error handling
    ///
    /// If all instances of a worker are created during startup, error handling can be side-stepped
    /// due to the previously outlined [guarantees](#guarantees).
    ///
    /// For workers that are built outside of main (e.g. from a parent worker), it's still
    /// recommended to  implement error handling, even if the parent worker itself is only
    /// constructed from `main()`.
    /// This is recommended as your architecture may change and the resulting regression might not
    /// present itself immediately.
    /// - **It's also very easy for child workers!** (see the previous example for reference)
    ///
    /// ```
    #[doc = include_str!("../doc/elme_proxy.rs")]
    /// use elme::shutdown::{ShutdownManager, TaskHandle};
    ///
    /// pub struct MyWorker {
    ///      task_handle: TaskHandle,
    /// }
    /// impl MyWorker {
    ///      pub fn new(task_handle: TaskHandle) -> Self {
    ///          Self { task_handle }
    ///      }
    ///      pub async fn run(self) {
    ///          // ...
    ///          # self.task_handle.wait_for_teardown_start().await;
    ///          # let _ = self.task_handle.manager().stop(0);
    ///      }
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let shutdown_mgr = ShutdownManager::default();
    ///     shutdown_mgr.inform_starting();
    ///
    ///     // since task is registered during startup (in the same thread) we can safely unwrap
    ///     let my_worker_handle = shutdown_mgr.register_task("My worker").unwrap();
    ///     let my_worker = MyWorker::new(my_worker_handle);
    ///
    ///     // done startup
    ///     shutdown_mgr.inform_started();
    ///     // start worker
    ///     tokio::spawn(async move { my_worker.run().await; });
    ///
    ///     // wait for teardown completion
    ///     shutdown_mgr.wait_for_teardown_done().await;
    /// }
    /// ```
    ///
    /// ## Initializing concurrent workers
    ///
    /// If a single handle is cloned to populate multiple instances of a worker in one go,
    /// the original handle should be manually dropped (or strategically scoped) to prevent
    /// lingering handles.
    ///
    /// **NOTE:** This scenario can be completely avoided by simply calling `register_task` from
    /// within the loop. This example is provided to emphasize the importance of handle lifetimes.
    ///
    /// ```
    /// # use std::error::Error;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn Error>> {
    #[doc = include_str!("../doc/elme_proxy.rs")]
    /// use elme::shutdown::ShutdownManager;
    ///
    /// const CONCURRENCY: usize = 4;
    ///
    /// let shutdown_mgr = ShutdownManager::default();
    /// shutdown_mgr.inform_starting()?;
    ///
    /// let mut workers = Vec::new();
    /// let handle = shutdown_mgr.register_task("Concurrent worker")?;
    /// for _ in (0..CONCURRENCY) {
    ///     let worker = MyWorker::new(handle.clone()); // handle is cloned
    ///     workers.push(worker);
    /// }
    /// drop(handle); // **CRITICAL** - handle must be manually dropped
    ///
    /// // done startup
    /// shutdown_mgr.inform_started()?;
    ///
    /// // run workers in background
    /// for worker in workers {
    ///     tokio::spawn(async move { worker.run().await; });
    /// }
    /// # Ok(())
    /// # }
    /// # use elme_shutdown::TaskHandle;
    /// # pub struct MyWorker { task_handle: TaskHandle }
    /// # impl MyWorker {
    /// #      pub fn new(task_handle: TaskHandle) -> Self { Self { task_handle } }
    /// #      pub async fn run(self) {
    /// #          self.task_handle.wait_for_teardown_start().await;
    /// #          let _ = self.task_handle.manager().stop(0);
    /// #      }
    /// # }
    /// ```
    pub fn register_task(&self, task_name: &'static str) -> Result<TaskHandle, RegisterError> {
        SharedState::register_task(&self.shared, task_name)
    }

    /// Returns a list of tasks and associated instance counts
    ///
    /// Does not include inactive tasks (tasks with an imstance count of `0`)
    /// - Occurs when a registered task has all [`TaskHandle`]s dropped
    #[must_use]
    pub fn task_list(&self) -> TrackedTaskList {
        let mut task_list = self.shared_lock().task_registry().as_task_list().into_active_filtered();
        task_list.sort_tasks();
        task_list
    }

    /// Total number of tasks (distinct task names) with `>= 1` instances.
    #[must_use]
    pub fn active_task_count(&self) -> usize {
        self.shared_lock().task_registry().active_task_count()
    }
    /// Returns `true` if there is at least 1 task instance active
    ///
    /// Or, more technically: if more than 1 `TaskHandle` hasn't been dropped,
    #[must_use]
    pub fn has_active_tasks(&self) -> bool {
        !self.shared_lock().task_registry().has_active_tasks()
    }

    /// Total number of task instances currently present
    ///
    /// Equivalent to the number of [`TaskHandle`]s that haven't been dropped.
    #[must_use]
    pub fn total_instance_count(&self) -> InstanceCount {
        self.shared_lock().task_registry().total_instance_count()
    }
    /// Total number of task instances currently present for a given task name
    ///
    /// Equivalent to the number of [`TaskHandle`]s for the given task name which haven't
    /// been dropped.
    #[must_use]
    pub fn task_instance_count(&self, task_name: &'static str) -> InstanceCount {
        self.shared_lock().task_registry().task_instance_count(task_name)
    }

    // !- Teardown futures

    /// Returns a [`Future`] which will be resolved once teardown starts.
    ///
    /// If teardown has already started, the future will resolve immediately.
    ///
    /// This is typically used by tasks to trigger their graceful stop logic.
    ///
    /// For convenience, [`TaskHandle`] also contains a
    /// [`wait_for_teardown_start()`](TaskHandle::wait_for_teardown_start) method
    /// providing identical functionality.
    pub fn wait_for_teardown_start(&self) -> impl Future<Output=()> {
        let token = self.shared_lock().teardown_start_token();
        token.cancelled_owned()
    }

    /// Returns a [`Future`] which will be resolved once teardown completes successfully (all
    /// `TaskHandle`'s dropped).
    /// If teardown has already finished, the future will resolve immediately.
    ///
    /// This is typically used from `main()` to delay exit/restart until all tasks have
    /// finished gracefully.
    ///
    /// Unlike [`wait_for_teardown_start()`](Self::wait_for_teardown_start), [`TaskHandle`] does not
    /// provide a `wait_for_teardown_done()` method, as this usage would almost always be an
    /// anti-pattern (the future will never complete because the associated handle likely remains in
    /// scope, preventing teardown from finishing).
    pub fn wait_for_teardown_done(&self) -> impl Future<Output=()> {
        let token = self.shared_lock().teardown_done_token();
        token.cancelled_owned()
    }

    // !- Teardown callbacks

    /// An optional callback/closure fn that is called once teardown completes.
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
    /// # Example
    ///
    /// ```
    /// # #[tokio::main]
    /// # pub async fn main() {
    #[doc = include_str!("../doc/elme_proxy.rs")]
    /// use elme::shutdown::ShutdownManager;
    ///
    /// let shutdown_manager = ShutdownManager::default()
    ///     .on_teardown(|stats| {
    ///         println!("Teardown timed-out in {}", stats.duration_text());
    ///     });
    /// # }
    /// ```
    ///
    /// # Usage
    ///
    /// The intended use-case is for alternative handling or reporting of teardown stats.
    ///
    /// This should **not** be used to handle cleanup / shutdown procedures.
    ///
    /// # Behavior
    ///
    /// The provided fn will only be executed if **all tasks are stopped gracefully
    /// before the timeout is reached**.
    ///
    /// If the teardown was triggered with a stop command, the application
    /// will terminate ~immediately after the provided fn returns.
    ///
    /// Otherwise - for a reload command - startup should begin ~immediately after.
    ///
    /// ---
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
    /// Supports builder-style method chaining (`mut` is not required).
    ///
    /// Replaces any fn's provided by prior invocations. To unset the callback,
    /// use [`unset_on_timeout()`](Self::unset_on_teardown).
    ///
    /// # Parameters
    ///
    /// The provided fn receives a single parameter: [`&TeardownTimeoutStats`](TeardownTimeoutStats).
    ///
    /// # Example
    ///
    /// ```
    /// # #[tokio::main]
    /// # pub async fn main() {
    #[doc = include_str!("../doc/elme_proxy.rs")]
    /// use elme::shutdown::ShutdownManager;
    ///
    /// let shutdown_manager = ShutdownManager::default()
    ///     .on_timeout(|stats| {
    ///         println!("Teardown timed-out in {}", stats.duration_text());
    ///         println!("{} instances did not stop in time.", stats.total_instances_timed_out());
    ///     });
    /// # }
    /// ```
    ///
    /// # Usage
    ///
    /// The intended use-case is for alternative handling or reporting of timeout stats.
    ///
    /// This should **not** be used to handle recovery attempts / cleanup / shutdown procedures.
    ///
    /// # Behavior
    ///
    /// The provided fn will only be executed if **the teardown timeout is reached before all
    /// tasks have gracefully stopped**.
    ///
    /// The application will **always** terminate ~immediately after this fn is called,
    /// regardless of the issued command.
    ///
    /// ---
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
