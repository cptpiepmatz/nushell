use crate::*;

// TODO: write docs here
pub static DATABASE_NOVA: ExperimentalOption = ExperimentalOption::new(&DatabaseNova);

// No documentation needed here since this type isn't public.
// The static above provides all necessary details.
struct DatabaseNova;

impl ExperimentalOptionMarker for DatabaseNova {
    const IDENTIFIER: &'static str = "database-nova";
    const DESCRIPTION: &'static str = "Rework of entire sqlite integration including commands.";
    const STATUS: Status = Status::OptIn;
    const SINCE: Version = (0, 0, 0);
    const ISSUE: u32 = 0;
}
