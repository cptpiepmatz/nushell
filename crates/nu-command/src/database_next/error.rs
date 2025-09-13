use std::string::FromUtf8Error;

use nu_protocol::{shell_error::io::IoError, Span, Type, Value};

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

    QueryStatement {
        sql: SqlString,
        span: Span,
        error: rusqlite::Error,
    },

    Iterate {
        sql: SqlString,
        index: usize,
        error: rusqlite::Error,
    },

    Unsupported {
        r#type: Type,
        span: Span,
    },

    Io(IoError),

    FromUtf8 {
        span: Span,
        error: FromUtf8Error,
    },
}