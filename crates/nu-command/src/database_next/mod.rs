use nu_protocol::engine::StateWorkingSet;

mod commands;
mod value;
mod error;

const SQLITE_MAGIC_BYTES: &[u8; 16] = b"SQLite format 3\0";

pub fn add_database_decls(working_set: &mut StateWorkingSet) {}

// TODO: provide database connection for history

// TODO: provide implementation for `stor` commands
