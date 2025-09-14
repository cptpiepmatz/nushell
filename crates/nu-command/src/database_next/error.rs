use std::{borrow::Cow, fmt::Debug, path::PathBuf, string::FromUtf8Error};

use nu_protocol::{shell_error::io::IoError, ShellError, Span, Type};

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

    Restore {
        path: PathBuf,
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
        value: rusqlite::types::Value,
        decl_type: DatabaseDeclType,
        span: Span,
    },
}
