#[cfg(unix)]
mod unix;

#[cfg(windows)]
mod windows;

use crate::{
    config::ShutdownConfig,
    manager::ShutdownManager,
    state::LockingSharedState,
};

use std::collections::HashSet;
use std::fmt;
use std::hash::Hash;
use std::process;
use std::sync::OnceLock;

// !- Statics

static ACTIVE: OnceLock<()> = OnceLock::new();

// !- Platform-agnostic traits

// While not required for functionality, traits are utilized here to ease
// cross-platform development by providing an implementation contract.
//
// This grants safety even without awareness of platform-specific impls

pub(crate) trait PlatformSignal:
    fmt::Debug + fmt::Display
    + Clone + Copy
    + PartialEq + Eq
    + Hash
{
    fn as_str(self) -> &'static str;

    fn into_action(self, options: &ShutdownConfig) -> Action;
}

trait Receiver {
    type Signal: PlatformSignal;

    fn new(signal: Self::Signal) -> Self;

    async fn listen(&mut self) -> Self::Signal;
}

trait PlatformSignalSet {
    type Signal: PlatformSignal;

    fn new() -> Self;

    async fn select(&mut self) -> Self::Signal;
}

// !- Action

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum Action {
    Reload,
    Stop,
    #[allow(dead_code)] // silence unused warn on windows
    Terminate,
    /// same as `Terminate`, but indicates an error (non-requested immediate quit)
    Kill,
}

// !- Signal handler state

#[derive(Debug, Clone)]
struct SignalHandlerState<S: PlatformSignal> {
    received: HashSet<S>,
}
impl<S: PlatformSignal> SignalHandlerState<S> {
    pub(crate) fn has_received(&self, signal: S) -> bool {
        self.received.contains(&signal)
    }
    pub(crate) fn mark_received(&mut self, signal: S) {
        self.received.insert(signal);
    }
}
impl<S: PlatformSignal> Default for SignalHandlerState<S> {
    fn default() -> Self {
        Self {
            received: HashSet::new()
        }
    }
}

// !- Signal handler

#[derive(Debug)]
pub(crate) struct SignalHandler<S: PlatformSignal> {
    state: SignalHandlerState<S>,
    shared: LockingSharedState,

    options: ShutdownConfig,
}
impl<S: PlatformSignal> SignalHandler<S> {
    const EXIT_CODE_FROM_SIGNAL: u8 = 0;

    fn new(shared: LockingSharedState) -> Self {
        let options = shared.lock().unwrap().options();
        Self {
            state: SignalHandlerState::default(),
            shared,
            options,
        }
    }
    fn new_initialized_impl(shared: LockingSharedState) -> Self {
        let handler = Self::new(shared);
        ACTIVE.set(()).expect("Global signal handler initialized only once!");

        handler
    }

    async fn run_for_platform<T: PlatformSignalSet<Signal = S>>(mut self) {
        let mut signal_set = T::new();

        loop {
            let signal_kind = signal_set.select().await;
            tracing::info!("Received signal: {signal_kind}");

            let mut action = signal_kind.into_action(&self.options);

            // handle term on 2nd signal receive
            if action == Action::Stop && self.options.terminate_on_second_signal() {
                if self.state.has_received(signal_kind) {
                    tracing::warn!("{signal_kind} signal was invoked for a 2nd time - terminating shortly...");
                    action = Action::Kill;
                } else {
                    self.state.mark_received(signal_kind);
                }
            }

            self.handle_action(action);
        }
    }

    fn handle_action(&self, action: Action) {
        match action {
            Action::Kill => {
                println!("Terminating now [FORCED]");
                process::exit(-1);
            }
            Action::Terminate => {
                tracing::warn!("Terminating now");
                process::exit(Self::EXIT_CODE_FROM_SIGNAL.into());
            }
            Action::Stop => {
                tracing::debug!("Dispatching stop");
                let manager = ShutdownManager::from(self.shared.clone());
                let _ = manager.stop(Self::EXIT_CODE_FROM_SIGNAL);

            }
            Action::Reload => {
                tracing::debug!("Dispatching reload");
                let manager = ShutdownManager::from(self.shared.clone());
                let _ = manager.reload();
            }
        }
    }
}

// ! Unix impl

#[cfg(unix)]
impl SignalHandler<unix::SignalType> {
    pub(crate) fn new_initialized(shared: LockingSharedState) -> Self {
        Self::new_initialized_impl(shared)
    }
    pub(crate) async fn run(self) {
        self.run_for_platform::<unix::SignalSet>().await;
    }
    pub(crate) fn run_in_background(self) {
        tokio::spawn(async move { self.run().await; });
    }
}

// ! Windows impl

#[cfg(windows)]
impl SignalHandler<windows::SignalType> {
    pub(crate) fn new_initialized(shared: LockingSharedState) -> Self {
        Self::new_initialized_impl(shared)
    }
    pub(crate) async fn run(self) {
        self.run_for_platform::<windows::SignalSet>().await;
    }
    pub(crate) fn run_in_background(self) {
        tokio::spawn(async move { self.run().await; });
    }
}
