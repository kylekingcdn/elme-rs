use std::fmt;

#[derive(Debug, Copy, Clone, Eq)]
pub enum Command {
    Stop(StopCommand),
    Reload,
}
impl Command {
    #[must_use]
    pub fn is_stop(&self) -> bool {
        matches!(self, Command::Stop {..})
    }
    #[must_use]
    pub fn is_reload(&self) -> bool {
        matches!(self, Command::Reload)
    }
}
impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stop(cmd)  => write!(f, "{cmd}"),
            Self::Reload => write!(f, "Reload"),
        }
    }
}
impl PartialEq for Command {
    fn eq(&self, other: &Self) -> bool {
        self.is_stop() == other.is_stop()
    }
}
impl From<StopCommand> for Command {
    fn from(stop_cmd: StopCommand) -> Self {
        Self::Stop(stop_cmd)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct StopCommand {
  pub exit_code: i32,
}
impl fmt::Display for StopCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Stop({})", self.exit_code)
    }
}

// !- Result

// Intentionally not a Result to force error handling (trigger kill)
#[derive(Debug, Clone)]
pub enum StopResult {
    Issued(StopCommand),
    IssuedPending(StopCommand),
    AlreadyIssued(StopCommand),
}
impl StopResult {
    #[must_use]
    pub fn was_issued(&self) -> bool {
        matches!(self, Self::Issued(_)) || matches!(self, Self::IssuedPending(_))
    }
    #[must_use]
    pub fn command(&self) -> StopCommand {
        match &self {
            Self::Issued(cmd) |
            Self::IssuedPending(cmd) |
            Self::AlreadyIssued(cmd) => *cmd,
        }
    }
    #[must_use]
    pub fn exit_code(&self) -> i32 {
        self.command().exit_code
    }
}

// Intentionally not a Result to force error handling (trigger kill))
#[derive(Debug, Clone)]
pub enum ReloadResult {
    Issued,
    IssuedPending,
    AlreadyIssued,
    Stopping(StopCommand),
}
impl ReloadResult {
    #[must_use]
    pub fn was_issued(&self) -> bool {
        matches!(self, Self::Issued) || matches!(self, Self::AlreadyIssued)
    }
}

#[derive(Debug, Clone)]
pub enum CommandResult {
    Stop(StopResult),
    Reload(ReloadResult),
}
impl CommandResult {
    #[must_use]
    pub fn was_issued(&self) -> bool {
        match &self {
            Self::Stop(res) => res.was_issued(),
            Self::Reload(res) => res.was_issued(),
        }
    }
}
impl From<StopResult> for CommandResult {
    fn from(res: StopResult) -> Self {
        Self::Stop(res)
    }
}
impl From<ReloadResult> for CommandResult {
    fn from(res: ReloadResult) -> Self {
        Self::Reload(res)
    }
}
