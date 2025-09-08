use std::{
    borrow::Cow,
    fmt::Debug,
    path::{Path, PathBuf},
};

use nu_engine::command_prelude::IoError;
use nu_protocol::{ShellError, Span};
use rusqlite::{DatabaseName, RowIndex, Statement};

use crate::database::values::sqlite::{DatabasePath, SqlInput};

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

    Prepare {
        sql: SqlInput,
        error: rusqlite::Error,
    },

    Execute {
        sql: SqlInput,
        error: rusqlite::Error,
    },

    // Failed to query SQLite database
    Query {
        sql: SqlInput,
        error: rusqlite::Error,
    },

    Iterate {
        sql: SqlInput,
        index: usize,
        error: rusqlite::Error,
    },

    Get {
        sql: Option<SqlInput>,
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
        match self {
            DatabaseError::Shell(shell_error) => shell_error,
            DatabaseError::NotASqliteFile { path } => ShellError::GenericError {
                error: "Not a SQLite database file".into(),
                msg: format!("'{}' is not a SQLite database file", path.display()),
                span: call_span.into(),
                help: None,
                inner: vec![],
            },
            DatabaseError::OpenConnection {
                path: DatabasePath::Path(path),
                error,
            } => ShellError::GenericError {
                error: "Failed to open SQLite connection from disk".into(),
                msg: error.to_string(),
                span: call_span.into(),
                help: None,
                inner: vec![ShellError::GenericError {
                    error: format!("Failed at path '{}'", path.item.display()),
                    msg: "Failed here".into(),
                    span: path.span.into(),
                    help: None,
                    inner: vec![],
                }],
            },
            DatabaseError::OpenConnection {
                path: DatabasePath::InMemory,
                error,
            } => ShellError::GenericError {
                error: "Failed to open SQLite standard connection in memory".into(),
                msg: error.to_string(),
                span: call_span.into(),
                help: None,
                inner: vec![],
            },
            DatabaseError::OpenConnection {
                path: DatabasePath::InMemoryCustom,
                error,
            } => ShellError::GenericError {
                error: "Failed to open SQLite custom connection in memory".into(),
                msg: error.to_string(),
                span: call_span.into(),
                help: None,
                inner: vec![],
            },
            DatabaseError::SetBusyHandler {
                path: DatabasePath::Path(path),
                error,
            } => ShellError::GenericError {
                error: "Failed to set busy handler SQLite connection from disk".into(),
                msg: error.to_string(),
                span: call_span.into(),
                help: None,
                inner: vec![ShellError::GenericError {
                    error: format!("Failed at path '{}'", path.item.display()),
                    msg: "Failed here".into(),
                    span: path.span.into(),
                    help: None,
                    inner: vec![],
                }],
            },
            DatabaseError::SetBusyHandler {
                path: DatabasePath::InMemory,
                error,
            } => ShellError::GenericError {
                error: "Failed to set busy handler for SQLite standard connection in memory".into(),
                msg: error.to_string(),
                span: call_span.into(),
                help: None,
                inner: vec![],
            },
            DatabaseError::SetBusyHandler {
                path: DatabasePath::InMemoryCustom,
                error,
            } => ShellError::GenericError {
                error: "Failed to set busy handler for SQLite custom connection in memory".into(),
                msg: error.to_string(),
                span: call_span.into(),
                help: None,
                inner: vec![],
            },
            DatabaseError::Prepare { sql, error } => ShellError::GenericError {
                error: "Failed to prepare statement".into(),
                msg: format!("Could not prepare `{sql}`"),
                span: sql.span().unwrap_or(call_span).into(),
                help: None,
                inner: vec![Self::related_err(error, Self::related_span(&sql, call_span))],
            },
            DatabaseError::Execute { sql, error } => ShellError::GenericError {
                error: "Failed to execute statement".into(),
                msg: format!("Could not execute `{sql}`"),
                span: sql.span().unwrap_or(call_span).into(),
                help: None,
                inner: vec![Self::related_err(error, Self::related_span(&sql, call_span))],
            },
            DatabaseError::Query { sql, error } => ShellError::GenericError {
                error: "Failed to query statement".into(),
                msg: format!("Could not query `{sql}`"),
                span: sql.span().unwrap_or(call_span).into(),
                help: None,
                inner: vec![Self::related_err(error, Self::related_span(&sql, call_span))],
            },
            DatabaseError::Iterate { sql, index, error } => ShellError::GenericError {
                error: "Failed to iterate rows".into(),
                msg: format!("Error at {index} for `{sql}`"),
                span: sql.span().unwrap_or(call_span).into(),
                help: None,
                inner: vec![Self::related_err(error, Self::related_span(&sql, call_span))],
            },
            DatabaseError::Get { sql, index, error } => ShellError::GenericError {
                error: "Failed to get column from row".into(),
                msg: match &sql {
                    Some(sql) => format!("Could not get {index:?} from row at `{sql}`"),
                    None => format!("Could not get {index:?} from row"),
                },
                span: sql.as_ref().and_then(|sql| sql.span()).unwrap_or(call_span).into(),
                help: None,
                inner: vec![Self::related_err(error, match sql {
                    Some(SqlInput::Spanned { .. }) => Some(call_span),
                    _ => None
                })],
            },
            DatabaseError::Backup {
                database_name,
                path,
                error,
            } => ShellError::GenericError {
                error: "Failed to backup SQLite database".into(),
                msg: format!(
                    "Could not backup '{database_name:?}' to '{}'",
                    path.display()
                ),
                span: call_span.into(),
                help: None,
                inner: vec![Self::related_err(error, None)],
            },
            DatabaseError::Restore {
                database_name,
                path,
                error,
            } => ShellError::GenericError {
                error: "Failed to restore SQLite database".into(),
                msg: format!(
                    "Could not restore '{database_name:?}' from '{}'",
                    path.display()
                ),
                span: call_span.into(),
                help: None,
                inner: vec![Self::related_err(error, None)],
            },
        }
    }

    fn related_err(err: rusqlite::Error, span: impl Into<Option<Span>>) -> ShellError {
        ShellError::GenericError {
            error: err.to_string(),
            msg: String::default(),
            span: span.into(),
            help: None,
            inner: vec![],
        }
    }

    fn related_span(sql_input: &SqlInput, call_span: Span) -> Option<Span> {
        match sql_input.span() {
            Some(_) => Some(call_span),
            None => None,
        }
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

    // Converting DatabaseError into a ShellError should only happen via into_shell_error.
    impl From<DatabaseError> for ShellError {
        fn from(value: DatabaseError) -> Self {
            panic!("Use into_shell_error instead");
        }
    }

    // ShellError should not be converted automatically to DatabaseError to ensure that all errors
    // we define in this module are either DatabaseError or passed from another function which
    // lives outside this module.
    impl From<ShellError> for DatabaseError {
        fn from(_: ShellError) -> Self {
            panic!("ShellError should not be converted automatically to DatabaseError");
        }
    }
}
