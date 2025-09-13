use std::string::FromUtf8Error;

use nu_protocol::{Span, shell_error::io::IoError};

use crate::database_next::plumbing::{sql::SqlString, storage::DatabaseStorage};

pub enum DatabaseError {
    OpenConnection {
        storage: DatabaseStorage,
        span: Span,
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

    Io(IoError),

    FromUtf8 {
        span: Span,
        error: FromUtf8Error,
    },
}