use crate::{
    manager::ShutdownManager,
    state::LockingSharedState,
};

use std::fmt;
use std::process;
use std::sync::{Mutex, OnceLock};
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::mpsc;

// !- Statics

static HANDLER: OnceLock<SignalHandler> = OnceLock::new();

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
}

// !- Signal handler

#[derive(Debug)]
pub(crate) struct SignalHandler {
    state: Mutex<SignalHandlerState>,

    shared: LockingSharedState,
}
impl SignalHandler {
    const EXIT_CODE_FROM_SIGNAL: u8 = 0;

    fn new(shared: LockingSharedState) -> Self {
        Self {
            state: Mutex::new(SignalHandlerState::default()),
            shared,
        }
    }
    pub(crate) fn new_initialized(shared: LockingSharedState) {
        let (tx, rx) = mpsc::channel(10);
        let handler = Self::new(shared);
        HANDLER.set(handler).expect("Global signal handler initialized only once!");

        // !- FIXME: use tokio select and use the branch result to specify the signal type
        let recv_hup = SignalReceiver::new(SignalType::Hup, tx.clone());
        let recv_int = SignalReceiver::new(SignalType::Int, tx.clone());
        let recv_term = SignalReceiver::new(SignalType::Term, tx.clone());
        let recv_quit = SignalReceiver::new(SignalType::Quit, tx);

        tokio::spawn(async move {
            let _ = tokio::join!(
                HANDLER.get().unwrap().receive_signals(rx),
                recv_hup.listen(),
                recv_int.listen(),
                recv_term.listen(),
                recv_quit.listen(),
            );
        });
    }
    #[allow(clippy::if_not_else)]
    async fn receive_signals(&self, mut rx: mpsc::Receiver<SignalType>) {
        while let Some(kind) = rx.recv().await {
            match kind {
                SignalType::Hup => {
                    tracing::info!("Received SIG{kind}");
                    tracing::debug!("Issuing reload (graceful stop without exit [re-init])");
                    self.handle_reload();
                },
                SignalType::Int => {
                    let is_first = !self.state.lock().unwrap().received_int;
                    if is_first {
                        self.state.lock().unwrap().received_int = true;
                        tracing::info!("Received first SIG{kind}.");
                        tracing::debug!("Issuing graceful shutdown.");
                        self.handle_stop();
                    } else {
                        tracing::warn!("Received second SIG{kind}");
                        tracing::error!("TERMINATING NOW");
                        process::exit(-1);
                    }
                },
                SignalType::Term => {
                    let is_first = !self.state.lock().unwrap().received_term;
                    if is_first {
                        self.state.lock().unwrap().received_term = true;
                        tracing::info!("Received first SIG{kind}.");
                        tracing::debug!("Issuing graceful shutdown.");
                        self.handle_stop();
                    } else {
                        tracing::warn!("Received second SIG{kind}");
                        tracing::error!("TERMINATING NOW");
                        process::exit(-1);
                    }
                },
                SignalType::Quit => {
                    println!("Received SIG{kind}");
                    println!("TERMINATING NOW");
                    process::exit(-1);
                },
            }
        }

        panic!("receive_signals() fn returning");
    }
    fn handle_reload(&self) {
        let _ = ShutdownManager::from(self.shared.clone()).reload();
    }
    fn handle_stop(&self) {
        let _ = ShutdownManager::from(self.shared.clone()).stop(Self::EXIT_CODE_FROM_SIGNAL);
    }
}

// !- Signal receiver

pub(crate) struct SignalReceiver {
    kind: SignalType,

    tx: mpsc::Sender<SignalType>,
}
impl SignalReceiver {
    fn new(kind: SignalType, tx: mpsc::Sender<SignalType>) -> Self {
        Self {
            kind,
            tx,
        }
    }
    async fn listen(self) {
        let mut stream = signal(self.kind.into()).unwrap_or_else(|_|
            panic!("failed to init signal receiver for {}", self.kind)
        );
        loop {
            stream.recv().await;
            println!("Received signal: {}", self.kind);
            if let Err(error) = self.tx.send(self.kind).await {
                println!("failed to dispatch signal notification. Error: {error:#?}");
                println!("TERMINATING NOW");
                process::exit(-1);
            }
        }
    }
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
