use super::{Action, PlatformSignal, PlatformSignalSet, Receiver};
use crate::config::ShutdownConfig;

use std::fmt;
use std::io;
use tokio::signal::windows::{
    ctrl_break, ctrl_c, ctrl_close, ctrl_logoff, ctrl_shutdown,
    CtrlBreak, CtrlC, CtrlClose, CtrlLogoff, CtrlShutdown,
};

// !- Signal type

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) enum SignalType {
    C,
    Break,
    Close,
    Logoff,
    Shutdown,
}
impl PlatformSignal for SignalType {
    fn as_str(self) -> &'static str {
        match self {
            Self::C => "C",
            Self::Break => "BREAK",
            Self::Close => "CLOSE",
            Self::Logoff => "LOGOFF",
            Self::Shutdown => "SHUTDOWN",
        }
    }
    fn into_action(self, options: &ShutdownConfig) -> Action {
        match self {
            Self::C |
            Self::Close |
            Self::Logoff |
            Self::Shutdown => Action::Stop,
            Self::Break => {
                if options.reload_support() && options.reload_on_ctrl_break() {
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

// !- Signal receiver

#[derive(Debug)]
pub(crate) struct SignalReceiver {
    kind: SignalType,
    signal: CtrlSignal,
}
impl Receiver for SignalReceiver {
    type Signal = SignalType;

    fn new(kind: SignalType) -> Self {
        let sig = CtrlSignal::try_new(kind).unwrap_or_else(|_|
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

// !- Signal set

pub(crate) struct SignalSet {
    c: SignalReceiver,
    break_sig: SignalReceiver,
    close: SignalReceiver,
    logoff: SignalReceiver,
    shutdown: SignalReceiver,
}
impl PlatformSignalSet for SignalSet {
    type Signal = SignalType;

    fn new() -> Self {
        Self {
            c: SignalReceiver::new(SignalType::C),
            break_sig: SignalReceiver::new(SignalType::Break),
            close: SignalReceiver::new(SignalType::Close),
            logoff: SignalReceiver::new(SignalType::Logoff),
            shutdown: SignalReceiver::new(SignalType::Shutdown),
        }
    }
    async fn select(&mut self) -> Self::Signal {
        tokio::select! {
            biased;
            s = self.shutdown.listen() => s,
            s = self.logoff.listen() => s,
            s = self.close.listen() => s,
            s = self.c.listen() => s,
            s = self.break_sig.listen() => s,
        }
    }
}

// !- Signal enum

#[derive(Debug)]
pub(crate) enum CtrlSignal {
    C(CtrlC),
    Break(CtrlBreak),
    Close(CtrlClose),
    Logoff(CtrlLogoff),
    Shutdown(CtrlShutdown),
}
impl CtrlSignal {
    pub(crate) fn try_new(signal: SignalType) -> Result<Self, io::Error> {
        let out = match signal {
            // FIXME: unwraps
            SignalType::C => ctrl_c()?.into(),
            SignalType::Break => ctrl_break()?.into(),
            SignalType::Close => ctrl_close()?.into(),
            SignalType::Logoff => ctrl_logoff()?.into(),
            SignalType::Shutdown => ctrl_shutdown()?.into(),
        };

        Ok(out)
    }
    pub(crate) async fn recv(&mut self) {
        match self {
            Self::C(signal) => signal.recv().await,
            Self::Break(signal) => signal.recv().await,
            Self::Close(signal) => signal.recv().await,
            Self::Logoff(signal) => signal.recv().await,
            Self::Shutdown(signal) => signal.recv().await,
        };
    }
}
impl From<CtrlC> for CtrlSignal {
    fn from(signal: CtrlC) -> Self {
        Self::C(signal)
    }
}
impl From<CtrlBreak> for CtrlSignal {
    fn from(signal: CtrlBreak) -> Self {
        Self::Break(signal)
    }
}
impl From<CtrlClose> for CtrlSignal {
    fn from(signal: CtrlClose) -> Self {
        Self::Close(signal)
    }
}
impl From<CtrlLogoff> for CtrlSignal {
    fn from(signal: CtrlLogoff) -> Self {
        Self::Logoff(signal)
    }
}
impl From<CtrlShutdown> for CtrlSignal {
    fn from(signal: CtrlShutdown) -> Self {
        Self::Shutdown(signal)
    }
}
