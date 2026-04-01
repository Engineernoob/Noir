mod manifest;
mod manager;
mod protocol;

pub use manager::{PluginCommandResult, PluginManager, PluginStartupSummary};
pub use protocol::{CommandExecutionContext, CursorPosition};
