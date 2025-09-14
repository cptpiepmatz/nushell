use std::{borrow::Cow, string::FromUtf8Error};

use nu_protocol::{Span, Type, Value, shell_error::io::IoError};

use crate::database_next::plumbing::{
    decl_type::DatabaseDeclType, sql::SqlString, storage::DatabaseStorage,
};

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
