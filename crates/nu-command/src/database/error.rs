use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use nu_engine::command_prelude::IoError;
use nu_protocol::{ShellError, Span};
use rusqlite::{DatabaseName, Row, RowIndex, Statement};

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
        error: rusqlite::Error,
    },

    // Failed to set busy handler for SQLite database
    SetBusyHandler {
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
        index: Box<dyn RowIndex>,
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

impl DatabaseError {
    pub(crate) fn get_sql(
        stmt: &Statement,
        raw_sql: impl Into<Cow<'static, str>>,
    ) -> Cow<'static, str> {
        stmt.expanded_sql()
            .map(Cow::Owned)
            .unwrap_or_else(|| raw_sql.into())
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
