#![doc="Shutdown shared state / inner data"]

use crate::{
    command::{Command, ReloadResult, StopCommand, StopResult},
    config::ShutdownConfig,
    task::{
        handle::TaskHandle,
        registry::TaskRegistry,
    },
    teardown::{HookDeps, TeardownMonitor, TeardownMonitorParams},
};

use std::process;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

// !- State enums

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LifecycleStage {
    Startup,
    Running,
    Teardown,
}
impl LifecycleStage {
    #[must_use]
    pub fn is_startup(&self) -> bool {
        *self == Self::Startup
    }
    #[must_use]
    pub fn is_running(&self) -> bool {
        *self == Self::Running
    }
    #[must_use]
    pub fn is_teardown(&self) -> bool {
        *self == Self::Teardown
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RunState {
    Starting,
    Ready,
    Reloading,
    Stopping,
}
impl RunState {
    #[must_use]
    pub fn is_starting(&self) -> bool {
        *self == Self::Starting
    }
    #[must_use]
    pub fn is_ready(&self) -> bool {
        *self == Self::Ready
    }
    #[must_use]
    pub fn is_reloading(&self) -> bool {
        *self == Self::Reloading
    }
    #[must_use]
    pub fn is_stopping(&self) -> bool {
        *self == Self::Stopping
    }
}

// !- Operation errors

#[derive(Debug, thiserror::Error)]
pub enum RegisterError {
    #[error("Failed to register task '{0}'. Currently tearing down.")]
    TearingDown(&'static str),
}

#[derive(Debug, thiserror::Error)]
pub enum InformStartingError {
    #[error("Cannot be called before teardown completes")]
    InvalidState,
    #[error("Inform starting was already called")]
    AlreadyStarting,
    #[error("Cannot be called after first startup without an issued reload command. Currently issued command: {0}")]
    SubsequentStartNonReload(Command),
}

#[derive(Debug, thiserror::Error)]
pub enum InformStartedError {
    #[error("inform_starting() must be called before inform_started()")]
    NotStarting,
}


/// Type alias for an  async mutable [`SharedState`].
///
/// Simply [`SharedState`] wrapped in a [`Mutex`] and then an [`Arc`].
///
/// Provided for the sole purpose of reducing type definition annoyances + imports..
pub(crate) type LockingSharedState = Arc<Mutex<SharedState>>;

/// Core shutdown state
///
/// Provides async access to core functions, state data, and messaging contexts.
///
/// While most shutdown logic is handled here, construction of types that use
/// [`SharedState`] must be handled in the parent context.
#[derive(Debug)]
pub(crate) struct SharedState {
    /// user provided options
    options: ShutdownConfig,

    startup_done_token: Option<CancellationToken>,
    teardown_start_token: CancellationToken,
    teardown_done_token: CancellationToken,

    /// retains current set of registered tasks to handle gracefully
    task_registry: TaskRegistry,

    /// currently issued command, clears upon ready state
    issued_command: Option<Command>,

    /// currently pending command
    ///
    /// commands issued during startup are temporarily stored
    /// as a pending command.
    ///
    /// This is done to help avoid inconsistent state, as it ensures that
    /// the lifecycle stage, and any dependent signals (`app_should_start`, `is_tearing_down`, etc)
    /// remain constant throughout startup.
    ///
    /// Once startup is finished (by calling `inform_started`), `pending_command` will become the `issued_command`,
    /// will begin to be handled
    pending_command: Option<Command>,

    hook_deps: HookDeps,
}
impl SharedState {
    pub(crate) fn new(options: ShutdownConfig) -> Self {
        Self {
            options,
            startup_done_token: None,
            teardown_start_token: CancellationToken::new(),
            teardown_done_token: CancellationToken::new(),
            task_registry: TaskRegistry::new(),
            issued_command: None,
            pending_command: None,
            hook_deps: HookDeps::new(),
        }
    }
    #[cfg(feature = "progress")]
    pub(crate) fn with_progress(
        options: ShutdownConfig,
        progress: indicatif::MultiProgress,
    ) -> Self {
        let mut state = Self::new(options);
        state.hook_deps = HookDeps::with_progress(progress);
        state
    }

    #[allow(dead_code)] // feature-dependant
    #[must_use]
    pub fn hook_deps(&self) -> &HookDeps {
        &self.hook_deps
    }

    pub fn _options(&self) -> ShutdownConfig {
        self.options
    }
    // gets wiped on teardown done
    // gets rebuilt on [`inform_starting`]
    pub fn _startup_done_token(&self) -> Option<CancellationToken> {
        self.startup_done_token.clone()
    }
    pub fn teardown_start_token(&self) -> CancellationToken {
        self.teardown_start_token.clone()
    }
    pub fn teardown_done_token(&self) -> CancellationToken {
        self.teardown_done_token.clone()
    }
    pub fn task_registry(&self) -> &TaskRegistry {
        &self.task_registry
    }
    pub fn register_task(shared: &LockingSharedState, task_name: &'static str) -> Result<TaskHandle, RegisterError> {
        let mut locked = shared.lock().unwrap();
        if locked.teardown_started() {
            Err(RegisterError::TearingDown(task_name))
        } else {
            locked.task_registry.register_task(task_name);
            let handle = TaskHandle::new(task_name, shared.clone());
            Ok(handle)
        }
    }
    pub fn unregister_task(shared: &LockingSharedState, task_name: &'static str) {
        shared.lock().unwrap().task_registry.unregister_task(task_name);
    }

    pub fn issued_command(&self) -> Option<Command> {
        self.issued_command
    }
    pub fn pending_command(&self) -> Option<Command> {
        self.pending_command
    }

    /// Returns true until [`inform_started`] is called
    ///
    /// This differs from [`startup_in_progress`], as this will return true prior to the call to [`inform_starting`].
    ///
    /// For reload commands, this will begin returning true once teardown has finished
    pub fn starting(&self) -> bool {
        // !- FIXME: return false if stop issued
        self.startup_done_token.as_ref().is_none_or(|t| !t.is_cancelled())
    }
    /// Returns true after calling [`inform_starting`], up until [`inform_started`] is called
    pub fn startup_in_progress(&self) -> bool {
        self.startup_done_token.as_ref().is_some_and(|t| !t.is_cancelled())
    }
    /// Returns true once [`inform_started`] is called
    pub fn startup_done(&self) -> bool {
        self.startup_done_token.as_ref().is_some_and(CancellationToken::is_cancelled)
    }
    /// Returns true after teardown has began
    ///
    /// Continues to return true once teardown finishes
    ///
    /// For reload commands, it will begin returning false again
    /// once [`inform_starting`] is called
    pub fn teardown_started(&self) -> bool {
        self.teardown_start_token.is_cancelled()
    }
    /// After a stop/reload command is issued, returns true once all tasks are finished
    pub fn _teardown_done(&self) -> bool {
        self.teardown_done_token.is_cancelled()
    }

    // current stage of app lifecycle
    pub fn lifecycle_stage(&self) -> LifecycleStage {
        if self.starting() {
            LifecycleStage::Startup
        } else if self.issued_command.is_none() {
            LifecycleStage::Running
        } else {
            LifecycleStage::Teardown
        }
    }
    // current state of app
    pub fn run_state(&self) -> RunState {
        match &self.issued_command {
            None => {
                if self.startup_done() {
                    RunState::Ready
                } else {
                    RunState::Starting
                }
            },
            Some(issued) => {
                match issued {
                    Command::Reload => RunState::Reloading,
                    Command::Stop {..} => RunState::Stopping,
                }
            }
        }
    }

    pub fn app_should_start(&self) -> bool {
        match self.issued_command {
            None |
            Some(Command::Reload) => true,
            Some(Command::Stop(..)) => false,
        }
    }

    pub fn inform_starting(shared: &LockingSharedState) -> Result<(),InformStartingError> {
        let mut locked = shared.lock().unwrap();

        if let Some(token) = &locked.startup_done_token {
            if token.is_cancelled() {
                Err(InformStartingError::InvalidState)
            } else {
                Err(InformStartingError::AlreadyStarting)
            }
        }
        // first start can be inferred by teardown done token.
        // it will be not cancelled on first start. the initial token cancelled check
        // covers cases of pre-mature calls. Therefore teardown_done can be used without concern for
        // valid lifecycle state when called.
        else {
            // indicates first start (issued command must be None)
            #[allow(clippy::if_not_else)]
            if !locked.teardown_done_token.is_cancelled() {
                // internal error, it shouldn't be possible to have a command issued on first startup (only pending)
                assert!(locked.issued_command.is_none(), "Unexpected state encountered. This is a bug. Please report it.");
            }
            // must be subsequent start, issued command must be reload
            else {
                // internal error, it shouldn't be possible to have a cancelled teardown done
                // token without having a command issued
                assert!(locked.issued_command.is_some(), "Unexpected state encountered. This is a bug. Please report it.");
                // user error - attempting startup after stop
                if let Some(cmd) = locked.issued_command && !cmd.is_reload() {
                    return Err(InformStartingError::SubsequentStartNonReload(cmd))
                }
            }
            // valid
            tracing::info!("App entering starting status");

            // rebuild startup token
            locked.startup_done_token = Some(CancellationToken::new());

            // rebuild teardown done token
            locked.teardown_done_token = CancellationToken::new();

            Ok(())
        }
    }
    pub fn inform_started(shared: &LockingSharedState) -> Result<(), InformStartedError> {
        let mut locked = shared.lock().unwrap();

        if locked.startup_in_progress() {
            tracing::info!("App entering started status");

            // clear issued command, lifecycle/run states transitions to running/ready
            locked.issued_command = None;

            // notify started
            locked.startup_done_token.as_ref().expect("should have a startup token").cancel();

            // must be called *after* the startup done token is cancelled
            let pending_cmd = locked.promote_pending_command();

            drop(locked);

            if let Some(cmd) = pending_cmd {
                tracing::info!("Handling previously pending command now: {cmd}");
                match cmd {
                    Command::Stop(..) |
                    Command::Reload => {
                        Self::start_teardown(shared);
                    }
                }
            }
            Ok(())
        } else {
            Err(InformStartedError::NotStarting)
        }
    }

    pub fn start_teardown(shared: &LockingSharedState) {
        let locked = shared.lock().unwrap();
        assert!(locked.startup_done_token.as_ref().is_some_and(CancellationToken::is_cancelled));
        assert!(!locked.teardown_start_token.is_cancelled());
        assert!(!locked.teardown_done_token.is_cancelled());

        locked.spawn_teardown_monitor(shared.clone());
        drop(locked);
        shared.lock().unwrap().teardown_start_token.cancel();
    }
    pub fn finish_teardown(shared: &LockingSharedState) {
        let mut locked = shared.lock().unwrap();
        assert!(locked.startup_done_token.as_ref().is_some_and(CancellationToken::is_cancelled));
        assert!(locked.teardown_start_token.is_cancelled());
        assert!(!locked.teardown_done_token.is_cancelled());
        assert!(!locked.task_registry.has_active_tasks());

        locked.teardown_done_token.cancel();

        // clear the teardown_start token, rebuild it upon inform starting
        locked.startup_done_token = None;
        // rebuild teardown started token
        locked.teardown_start_token = CancellationToken::new();

        locked.task_registry.rebuild_registration_channel();
    }

    /// shared will be locked during monitor init, only to be used to call [`finish_teardown`] after starting
    fn spawn_teardown_monitor(&self, shared: LockingSharedState) {
        let params = TeardownMonitorParams {
            options: self.options,
            initial_tasks: self.task_registry.task_map().clone(),
            registration_rx: self.task_registry.registration_rx(),
            hook_deps: self.hook_deps.clone(),
        };
        // init teardown monitor
        let monitor = TeardownMonitor::new(params);

        // run teardown monitor in background
        tokio::spawn(async move {
            // tracing::warn!("STARTING TEARDOWN MONITOR NOW");
            let teardown_res = monitor.start().await;
            match teardown_res {
                Ok(stats) => {
                    let task_count = stats.total_tasks();
                    tracing::info!(
                        started_at=?stats.started_at(),
                        finished_at=?stats.finished_at(),
                        elapsed_ms=stats.duration().as_millis(),
                        task_count,
                        "Teardown finished ({task_count} tasks stopped gracefully in {})",
                        stats.duration_text(),
                    );
                },
                Err(stats) => {
                    // let finished_list = stats.tasks.0.iter().filter(|t| t.is_fully_transitioned()).collect::<Vec<_>>();
                    tracing::error!("Teardown timed out while waiting for tasks to gracefully stop.");
                    tracing::trace!("Full teardown timeout stats:\n{stats:#?}");
                    tracing::warn!("Terminating now.");
                    process::exit(1);
                }
            }

            Self::finish_teardown(&shared);
        });
    }

    // if we're in startup, we should compare against the pending command, as startup is
    // immutable and once complete, the pending command will be loaded as the issued command
    fn loaded_command(&self) -> Option<Command> {
        if self.startup_in_progress() {
            self.pending_command
        } else {
            self.issued_command
        }
    }

    pub fn reload(&mut self) -> ReloadResult {
        tracing::trace!("Attempting to issue reload command");
        match self.loaded_command() {
            None => {
                let cmd = Command::Reload;
                if self.starting() {
                    self.pending_command = Some(cmd);
                    tracing::info!(%cmd, "Storing reload command as pending");
                    ReloadResult::IssuedPending
                } else {
                    self.issued_command = Some(cmd);
                    tracing::info!(%cmd, "Storing reload command as issued");
                    ReloadResult::Issued

                }
            },
            Some(Command::Reload) => ReloadResult::AlreadyIssued,
            Some(Command::Stop(stop_cmd)) => ReloadResult::Stopping(stop_cmd),
        }
    }
    pub fn stop(&mut self, exit_code: i32) -> StopResult {
        tracing::trace!(exit_code, "Attempting to issue stop command");
        match self.loaded_command() {
            None |
            Some(Command::Reload) => {
                let cmd = StopCommand { exit_code };
                if self.startup_in_progress() {
                    self.pending_command = Some(cmd.into());
                    tracing::info!(%cmd, "Storing stop command as pending");
                    StopResult::IssuedPending(cmd)
                } else {
                    self.issued_command = Some(cmd.into());
                    tracing::info!(%cmd, "Storing stop command as issued");
                    StopResult::Issued(cmd)
                }
            },
            Some(Command::Stop(stop_cmd)) => StopResult::AlreadyIssued(stop_cmd),
        }
    }

    /// promotes the pending command (if one exists), to issued
    fn promote_pending_command(&mut self) -> Option<Command> {
        if let Some(pending) = self.pending_command {
            assert!(self.startup_done(), "Pending commands can't be processed during startup");
            assert!(self.issued_command.is_none(), "Pending commands shouldn't coexist with issued commands outside of startup");
            tracing::info!(cmd=%pending, "Processing pending command that was previously issued: {pending}");
            self.issued_command = self.pending_command;
            self.pending_command = None;

            self.issued_command
        } else {
            None
        }
    }
}
