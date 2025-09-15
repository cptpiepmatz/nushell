use std::{
    borrow::Cow,
    fmt::{Debug, Display},
    path::PathBuf,
    string::FromUtf8Error,
};

use nu_protocol::{
    ShellError, Span, Type,
    shell_error::{io::IoError, location::Location},
};

use crate::database_next::plumbing::{
    decl_type::DatabaseDeclType, sql::SqlString, storage::DatabaseStorage,
};

#[derive(Debug)]
pub enum DatabaseError {
    // rare cases, only when nothing to do with database
    Shell(ShellError),

    OpenConnection {
        storage: DatabaseStorage,
        span: Span,
        error: rusqlite::Error,
    },

    OpenInternalConnection {
        storage: DatabaseStorage,
        location: Location,
        error: rusqlite::Error,
    },

    Promote {
        path: PathBuf,
        span: Span,
        error: rusqlite::Error,
    },

    Deserialize {
        call_span: Span,
        value_span: Span,
        error: rusqlite::Error,
    },

    PrepareStatement {
        sql: SqlString,
        span: Span,
        error: rusqlite::Error,
    },

    ExecuteStatement {
        sql: SqlString,
        span: Span,
        error: rusqlite::Error,
    },

    QueryStatement {
        sql: SqlString,
        span: Span,
        error: rusqlite::Error,
    },

    Iterate {
        sql: SqlString,
        index: usize,
        span: Span,
        error: rusqlite::Error,
    },

    Get {
        sql: SqlString,
        index: String,
        span: Span,
        error: rusqlite::Error,
    },

    Unsupported {
        r#type: Type,
        span: Span,
    },

    // mark this variant as deprecated to find missing pieces
    Todo {
        msg: Cow<'static, str>,
        span: Span,
    },

    Io(IoError),

    FromUtf8 {
        span: Span,
        error: FromUtf8Error,
    },

    InvalidDeclType {
        rusqlite_type: rusqlite::types::Type,
        decl_type: DatabaseDeclType,
        span: Span,
    },
}

fn generic_error(
    error: impl ToString,
    msg: impl ToString,
    span: impl Into<Option<Span>>,
    rusqlite_error: impl Into<Option<rusqlite::Error>>,
) -> ShellError {
    ShellError::GenericError {
        error: error.to_string(),
        msg: msg.to_string(),
        span: span.into(),
        help: None,
        inner: rusqlite_error
            .into()
            .map(|error| ShellError::GenericError {
                error: error.to_string(),
                msg: "".into(),
                span: None,
                help: None,
                inner: vec![],
            })
            .into_iter()
            .collect(),
    }
}

// TODO: for SqlString uses, also use the span/location of them
impl From<DatabaseError> for ShellError {
    fn from(error: DatabaseError) -> Self {
        match error {
            DatabaseError::Shell(shell_error) => shell_error,
            DatabaseError::OpenConnection {
                storage,
                span,
                error,
            } => generic_error(
                "Open connection to database failed",
                format!("Failed to open to {}", storage.as_path().display()),
                span,
                error,
            ),
            DatabaseError::OpenInternalConnection {
                storage,
                location: _, // TODO: handle this location properly
                error,
            } => generic_error(
                "Open internal connection to database failed",
                format!("Failed to open to {}", storage.as_path().display()),
                None,
                error,
            ),
            DatabaseError::Promote { path, span, error } => generic_error(
                "Promoting database connection failed",
                format!(
                    "Promoting {} into in-memory database failed",
                    path.display()
                ),
                span,
                error,
            ),
            DatabaseError::Deserialize {
                call_span,
                value_span,
                error,
            } => ShellError::GenericError {
                error: "Deserializing database failed".into(),
                msg: "Failed to deserialize database".into(),
                span: Some(call_span),
                help: None,
                inner: vec![ShellError::GenericError {
                    error: "Deserialization failed on a value".into(),
                    msg: error.to_string(),
                    span: Some(value_span),
                    help: None,
                    inner: vec![],
                }],
            },
            DatabaseError::PrepareStatement { sql, span, error } => generic_error(
                "Preparing statement failed",
                format!("Error preparing {:?}", sql.as_str()),
                span,
                error,
            ),
            DatabaseError::ExecuteStatement { sql, span, error } => generic_error(
                "Executing statement failed",
                format!("Error executing {:?}", sql.as_str()),
                span,
                error,
            ),
            DatabaseError::QueryStatement { sql, span, error } => generic_error(
                "Querying statement failed",
                format!("Error querying {:?}", sql.as_str()),
                span,
                error,
            ),
            DatabaseError::Iterate {
                sql,
                index,
                span,
                error,
            } => generic_error(
                "Iterating over database rows failed",
                format!("Error at {index} for {:?}", sql.as_str()),
                span,
                error,
            ),
            DatabaseError::Get {
                sql,
                index,
                span,
                error,
            } => generic_error(
                "Getting value from database row failed",
                format!("Error at {index:?} for {:?}", sql.as_str()),
                span,
                error,
            ),
            DatabaseError::Unsupported { r#type, span } => generic_error(
                "Unsupported type for database",
                format!("The type {} is not supported", r#type),
                span,
                None,
            ),
            DatabaseError::Todo { msg, span } => generic_error("Database To-Do", msg, span, None),
            DatabaseError::Io(io_error) => ShellError::Io(io_error),
            DatabaseError::FromUtf8 { span, error } => generic_error(
                "Encountered non-utf8 strings in database",
                error,
                span,
                None,
            ),
            DatabaseError::InvalidDeclType {
                rusqlite_type,
                decl_type,
                span,
            } => generic_error(
                "Invalid declaration type",
                format!("{} cannot be deserialized as {}", rusqlite_type, decl_type),
                span,
                None,
            ),
        }
    }
}
