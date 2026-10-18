use super::{Action, PlatformSignal, PlatformSignalSet, Receiver};
use crate::config::ShutdownConfig;

use std::fmt;
use tokio::signal::unix::{signal, Signal, SignalKind};

// !- Signal type

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) enum SignalType {
    Hup,
    Int,
    Term,
    Quit,
}
impl PlatformSignal for SignalType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Hup => "HUP",
            Self::Int => "INT",
            Self::Term => "TERM",
            Self::Quit => "QUIT",
        }
    }
    fn into_action(self, options: &ShutdownConfig) -> Action {
        match self {
            Self::Quit => Action::Terminate,
            Self::Int |
            Self::Term => Action::Stop,
            Self::Hup => {
                if options.reload_support() {
                    Action::Reload
                } else {
                    Action::Stop
                }
            }
        }
    }
}
impl fmt::Display for SignalType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
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

// !- Signal receiver

#[derive(Debug)]
pub(crate) struct SignalReceiver {
    kind: SignalType,
    signal: Signal,
}
impl Receiver for SignalReceiver {
    type Signal = SignalType;

    fn new(kind: Self::Signal) -> Self {
        let sig = signal(kind.into()).unwrap_or_else(|_|
            panic!("Failed to register signal receiver for {kind}")
        );
        Self {
            signal: sig,
            kind,
        }
    }
    async fn listen(&mut self) -> Self::Signal {
        self.signal.recv().await;
        self.kind
    }
}

// !- Signal set

pub(crate) struct SignalSet {
    hup: SignalReceiver,
    int: SignalReceiver,
    term: SignalReceiver,
    quit: SignalReceiver,
}
impl PlatformSignalSet for SignalSet {
    type Signal = SignalType;

    fn new() -> Self {
        Self {
            hup: SignalReceiver::new(SignalType::Hup),
            int: SignalReceiver::new(SignalType::Int),
            term: SignalReceiver::new(SignalType::Term),
            quit: SignalReceiver::new(SignalType::Quit),
        }
    }
    async fn select(&mut self) -> SignalType {
        tokio::select! {
            biased;
            s = self.quit.listen() => s,
            s = self.int.listen() => s,
            s = self.term.listen() => s,
            s = self.hup.listen() => s,
        }
    }
}
