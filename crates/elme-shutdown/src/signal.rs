use crate::{
    config::ShutdownConfig,
    manager::ShutdownManager,
    state::LockingSharedState,
};

use std::fmt;
use std::process;
use std::sync::{Mutex, OnceLock};
use tokio::signal::unix::{signal, Signal, SignalKind};

// !- Statics

static ACTIVE: OnceLock<()> = OnceLock::new();

// !- Signal handler state

/// Uses internal state to track multiple invocations of a signal.
///
/// Does not use the shared state's
/// [`IssuedCommand`](crate::command::IssuedCommand) member for tracking.
/// This is because issued signals should be counted independently.
///
/// Further, issuing a stop internally (from app code) should not cause an
/// immediate hard kill on the first signal (e.g. from `ctrl+c`).
#[derive(Debug, Copy, Clone, Default)]
struct SignalHandlerState {
    pub received_int: bool,
    pub received_term: bool,
    pub received_hup: bool,
}

// !- Signal handler

#[derive(Debug)]
pub(crate) struct SignalHandler {
    state: Mutex<SignalHandlerState>,
    shared: LockingSharedState,

    options: ShutdownConfig,
}
impl SignalHandler {
    const EXIT_CODE_FROM_SIGNAL: u8 = 0;

    fn new(shared: LockingSharedState) -> Self {
        let options = shared.lock().unwrap().options();
        Self {
            state: Mutex::new(SignalHandlerState::default()),
            shared,
            options,
        }
    }
    pub(crate) fn new_initialized(shared: LockingSharedState) -> Self {
        let handler = Self::new(shared);
        ACTIVE.set(()).expect("Global signal handler initialized only once!");

        handler
    }
    pub(crate) fn run_in_background(self) {
        tokio::spawn(async move { self.run().await; });
    }
    pub(crate) async fn run(self) {
        let mut hup = SignalReceiver::new(SignalType::Hup);
        let mut int = SignalReceiver::new(SignalType::Int);
        let mut term = SignalReceiver::new(SignalType::Term);
        let mut quit = SignalReceiver::new(SignalType::Quit);

        loop {
            let kind = tokio::select! {
                biased;
                s = quit.listen() => s,
                s = int.listen() => s,
                s = term.listen() => s,
                s = hup.listen() => s,
            };
            tracing::info!("Received SIG{kind}");
            let action = self.resolve_action(kind);
            self.handle_action(action);
        }
    }
    fn resolve_action(&self, kind: SignalType) -> Action {
        let mut got_second = false;
        let action = match kind {
            SignalType::Quit => {
                Action::Terminate
            }

            SignalType::Int => {
                if self.options.terminate_on_second_signal() &&
                   self.state.lock().unwrap().received_int {
                    got_second = true;
                }
                self.state.lock().unwrap().received_int = true;
                Action::Stop
            }

            SignalType::Term => {
                if self.options.terminate_on_second_signal() &&
                   self.state.lock().unwrap().received_term {
                    got_second = true;
                }
                self.state.lock().unwrap().received_term = true;
                Action::Stop
            }

            SignalType::Hup => {
                if self.options.reload_support() {
                    Action::Reload
                } else {
                    if self.options.terminate_on_second_signal() &&
                       self.state.lock().unwrap().received_hup {
                        got_second = true;
                    }
                    self.state.lock().unwrap().received_hup = true;
                    Action::Stop
                }
            }
        };

        // override to Kill, log warning
        if got_second {
            tracing::warn!("SIG{kind} was invoked for a 2nd time - terminating shortly...");
            Action::Kill
        } else {
            action
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

// !- Signal receiver

#[derive(Debug)]
pub(crate) struct SignalReceiver {
    kind: SignalType,
    signal: Signal,
}
impl SignalReceiver {
    fn new(kind: SignalType) -> Self {
        let sig = signal(kind.into()).unwrap_or_else(|_|
            panic!("Failed to register signal receiver for {kind}")
        );
        Self {
            signal: sig,
            kind,
        }
    }
    async fn listen(&mut self) -> SignalType {
        self.signal.recv().await;
        self.kind
    }
}

// !- Action

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum Action {
    Reload,
    Stop,
    Terminate,
    /// same as `Terminate`, but indicates an error (non-requested immediate quit)
    Kill,
}

// !- Signal kind

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum SignalType {
    Hup,
    Int,
    Term,
    Quit,
}
impl fmt::Display for SignalType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Hup => "HUP",
            Self::Int => "INT",
            Self::Term => "TERM",
            Self::Quit => "QUIT",
        };
        write!(f, "{name}")
    }
}
impl From<SignalType> for SignalKind {
    fn from(sig: SignalType) -> Self {
        match sig {
            SignalType::Hup => Self::hangup(),
            SignalType::Int => Self::interrupt(),
            SignalType::Term => Self::terminate(),
            SignalType::Quit => Self::quit(),
        }
    }
}
