use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    fmt::Debug,
};

use nu_engine::command_prelude::IoError;
use nu_protocol::{ShellError, Span};
use rusqlite::{DatabaseName, RowIndex, Statement};

#[derive(Debug)]
pub enum DatabaseError {
    /// Non-database errors.
    ///
    /// This variant allows easy piping through other errors outside the database context.
    /// The [`Into`] impl for this just extracts.
    Shell(ShellError),

    NotASqliteFile {
        path: PathBuf,
    },

    // Failed to open SQLite database from open_connection
    OpenConnection {
        path: DatabasePath,
        error: rusqlite::Error,
    },

    // Failed to set busy handler for SQLite database
    SetBusyHandler {
        path: DatabasePath,
        error: rusqlite::Error,
    },

    PrepareConnection {
        sql: Cow<'static, str>,
        error: rusqlite::Error,
    },

    Prepare {
        sql: Cow<'static, str>,
        error: rusqlite::Error,
    },

    Execute {
        sql: Cow<'static, str>,
        error: rusqlite::Error,
    },

    Query {
        sql: Cow<'static, str>,
        error: rusqlite::Error,
    },

    Iterate {
        sql: Cow<'static, str>,
        index: usize,
        error: rusqlite::Error,
    },

    Get {
        sql: Option<Cow<'static, str>>,
        index: Box<dyn RowIndexDebug>,
        error: rusqlite::Error,
    },

    Backup {
        database_name: DatabaseName<'static>, // no usages of DatabaseName::Attached, so we're fine
        path: Cow<'static, Path>,
        error: rusqlite::Error,
    },

    Restore {
        database_name: DatabaseName<'static>,
        path: Cow<'static, Path>,
        error: rusqlite::Error,
    },
}

#[derive(Debug)]
pub enum DatabasePath {
    Path(PathBuf),
    // "Failed to open SQLite standard connection in memory"
    Memory,
    // "Failed to open SQLite custom connection in memory"
    // "Failed to set busy handler for SQLite custom connection in memory"
    MemoryCustom,
}

trait RowIndexDebug: RowIndex + Debug {}
impl<T: RowIndex + Debug> RowIndexDebug for T {}

impl DatabaseError {
    pub(crate) fn get_sql(
        stmt: &Statement,
        raw_sql: impl Into<Cow<'static, str>>,
    ) -> Cow<'static, str> {
        stmt.expanded_sql()
            .map(Cow::Owned)
            .unwrap_or_else(|| raw_sql.into())
    }

    pub fn into_shell_error(self, call_span: Span) -> ShellError {
        todo!()
    }
}

impl From<DatabaseError> for ShellError {
    fn from(value: DatabaseError) -> Self {
        todo!()
    }
}

// Explicitly allow passing through io errors as they nowadays usually provide enough infos.
impl From<IoError> for DatabaseError {
    fn from(error: IoError) -> Self {
        Self::Shell(ShellError::Io(error))
    }
}

#[cfg(test)]
mod assert_no_impl {
    use super::*;

    // ShellError should not be converted automatically to DatabaseError to ensure that all errors
    // we define in this module are either DatabaseError or passed from another function which
    // lives outside this module.
    impl From<ShellError> for DatabaseError {
        fn from(_: ShellError) -> Self {
            panic!("ShellError should not be converted automatically to DatabaseError");
        }
    }
}
