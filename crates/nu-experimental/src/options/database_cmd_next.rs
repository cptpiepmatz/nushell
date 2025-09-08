use crate::*;

/// Use new implementation of the database-related commands.
///
/// This replaces the old implementation of:
/// - `into sqlite`
/// - `query db`
/// - `query`
/// - `schema`
/// 
/// And in the future of the `stor` commands.
///
/// The new implementation is built completely separately next to the old one, so any changes
/// shouldn't affect the old command.
///
/// Enabling this command might replace commands with other names and therefore possibly breaking
/// scripts.
pub static DATABASE_CMD_NEXT: ExperimentalOption = ExperimentalOption::new(&DatabaseCmdNext);

// No documentation needed here since this type isn't public.
// The static above provides all necessary details.
struct DatabaseCmdNext;

impl ExperimentalOptionMarker for DatabaseCmdNext {
    const IDENTIFIER: &'static str = "database-cmd-next";
    const DESCRIPTION: &'static str = concat!(
        "Use a new implementation of the database-related commands. ",
        "May contain breaking changes."
    );
    const STATUS: Status = Status::OptIn;
}
