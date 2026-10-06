use std::fmt;

/// Represents the set of supported commands
#[derive(Debug, Copy, Clone, Eq)]
pub enum Command {
    /// The `Stop` command, contains parameters required to call `Stop`
    Stop(StopCommand),
    /// The `Reload` command
    Reload,
}
impl Command {
    /// True if matches `Stop`
    #[must_use]
    pub fn is_stop(&self) -> bool {
        matches!(self, Command::Stop {..})
    }
    /// True if matches `Reload`
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

/// The parameters required to issue a `Stop` command
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct StopCommand {
    /// The code returned by the program upon exit
    pub exit_code: u8,
}
impl fmt::Display for StopCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Stop({})", self.exit_code)
    }
}

// !- Result

/// The possible outcomes of sending a `Stop` command.
#[derive(Debug, Clone)]
pub enum StopResult {
    /// Stop issued and stored as `issued_command`
    Issued(StopCommand),
    /// Replaced the Reload command stored under `issued_command`
    ///
    /// Only possible if the Reload startup hasn't begun yet
    IssuedUpgrade(StopCommand),
    /// Stop issued as pending and stored as `pending_command`
    Pending(StopCommand),
    /// Replaced the Reload command stored under `pending_command`
    PendingUpgrade(StopCommand),
    /// Stop has already been requested and is stored as either issued or pending
    ///
    /// The contained [`StopCommand`] is from the previously invoked `stop()` call.
    AlreadyIssued(StopCommand),
}
impl StopResult {
    /// Returns `true` for all cases except for `AlreadyIssued`
    #[must_use]
    pub fn was_issued(&self) -> bool {
        !matches!(self, Self::AlreadyIssued(_))
    }

    /// Returns the inner `StopCommand` contained by each variant
    #[must_use]
    pub fn command(&self) -> StopCommand {
        match &self {
            Self::Issued(cmd) |
            Self::IssuedUpgrade(cmd) |
            Self::Pending(cmd) |
            Self::PendingUpgrade(cmd) |
            Self::AlreadyIssued(cmd) => *cmd,
        }
    }

    /// Returns the exit code of the inner `StopCommand` contained by each variant
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        self.command().exit_code
    }
}

/// The possible outcomes of sending a `Reload` command.
#[derive(Debug, Clone)]
pub enum ReloadResult {
    /// Reload issued and stored as `issued_command`
    Issued,
    /// Reload issued as pending and stored as `pending_command`
    Pending,
    /// Reload has already been requested and is stored as either issued or pending
    AlreadyIssued,
    /// Stop has previously been requested, Reload is no longer permitted
    Stopping(StopCommand),
    /// Reload support has been explicitly disabled by [`ShutdownConfig::reload_support`](crate::ShutdownConfig::reload_support)
    Disabled,
}
impl ReloadResult {
    /// Returns `true` when `Issued` or `Pending`
    #[must_use]
    pub fn was_issued(&self) -> bool {
        matches!(self, Self::Issued) ||
        matches!(self, Self::Pending)
    }
}

/// Represents the possible result types for each issued `Command`
#[derive(Debug, Clone)]
pub enum CommandResult {
    /// Contains the type returned by [`stop()`](crate::ShutdownManager::stop)
    Stop(StopResult),
    /// Contains the type returned by [`reload()`](crate::ShutdownManager::reload)
    Reload(ReloadResult),
}
impl CommandResult {
    /// - If the variant is `CommandResult::Stop`:
    ///   - Passes through to [`StopResult::was_issued`]
    /// - If the variant is `CommandResult::Reload`:
    ///   - Passes through to [`ReloadResult::was_issued`]
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
