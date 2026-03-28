use std::{
    borrow::Cow,
    fmt::{Debug, Display},
    path::PathBuf,
    string::FromUtf8Error,
};

use nu_protocol::{
    ShellError, Span, Type, Value,
    shell_error::{ErrorSite, generic::GenericError, io::IoError},
};
use nu_utils::location::Location;
use thiserror::Error;

use crate::database_nova::plumbing::{
    decl_type::DatabaseDeclType, list::DatabaseList, name::DatabaseName, sql::SqlString,
    storage::DatabaseStorage, table::DatabaseTableName,
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

    DatabaseNotFound {
        name: DatabaseName,
        database_list: DatabaseList,
        span: Span,
    },

    TableNotFound {
        name: DatabaseName,
        table: DatabaseTableName,
        tables: Vec<DatabaseTableName>,
        span: Span,
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

    CannotConvertIntoDb {
        value: Value,
        expected: Type,
        span: Span,
    },

    // TODO: mark this variant as deprecated to find missing pieces
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
    code: impl Into<Cow<'static, str>>,
    error: impl Into<Cow<'static, str>>,
    msg: impl Into<Cow<'static, str>>,
    site: impl Into<ErrorSite>,
    rusqlite_error: impl Into<Option<rusqlite::Error>>,
) -> ShellError {
    let err = GenericError::new_with_site(error, msg, site.into()).with_code(code);
    ShellError::Generic(match rusqlite_error.into() {
        None => err,
        Some(rusqlite_error) => err.with_source(PlainError::new(rusqlite_error)),
    })
}

#[derive(Debug, Error)]
#[error("{0}")]
struct PlainError(String);

impl PlainError {
    pub fn new(content: impl Display) -> Self {
        let initial = content.to_string();
        let mut chars = initial.chars();
        let inner = match chars.next() {
            None => String::new(),
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        };
        Self(inner)
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
                "nu::shell::database::open_connection",
                "Open connection to database failed",
                format!("Failed to open to {}", storage.connection_path().display()),
                span,
                error,
            ),

            DatabaseError::OpenInternalConnection {
                storage,
                location,
                error,
            } => generic_error(
                "nu::shell::database::open_connection",
                "Open internal connection to database failed",
                format!("Failed to open to {}", storage.connection_path().display()),
                location,
                error,
            ),

            DatabaseError::DatabaseNotFound {
                name,
                database_list,
                span: value_span,
            } => {
                let (name, name_site) = name.into_parts();
                let did_you_mean =
                    nu_protocol::did_you_mean(database_list.iter().map(|entry| &entry.name), &name);
                let msg = format!("Could not find {:?} in database system", name);
                let inner = match (name_site, did_you_mean) {
                    (ErrorSite::Span(span), Some(suggestion)) => ShellError::DidYouMeanCustom {
                        msg: msg.clone(),
                        suggestion,
                        span,
                    },
                    (site, _) => ShellError::Generic(GenericError::new_with_site(
                        "Database not found",
                        format!("Could not find {:?} in database system", name),
                        site,
                    )),
                };
                ShellError::Generic(
                    GenericError::new(
                        "Database system does not contain expected database",
                        msg,
                        value_span,
                    )
                    .with_inner([inner]),
                )
            }

            DatabaseError::TableNotFound {
                name,
                table,
                tables,
                span: value_span,
            } => {
                let (name, name_site) = name.into_parts();
                let did_you_mean =
                    nu_protocol::did_you_mean(tables.iter().map(|entry| entry.as_str()), &name);
                let msg = format!("Could not find {:?}.{:?} in database", name, table.as_str());
                let inner = match (name_site, did_you_mean) {
                    (ErrorSite::Span(name_span), Some(suggestion)) => ShellError::DidYouMean {
                        suggestion,
                        span: name_span,
                    },
                    (site, _) => ShellError::Generic(GenericError::new_with_site(
                        "Table not found",
                        msg.clone(),
                        site,
                    )),
                };
                ShellError::Generic(
                    GenericError::new("Database does not contain expected table", msg, value_span)
                        .with_inner([inner]),
                )
            }

            DatabaseError::Promote { path, span, error } => generic_error(
                "nu::shell::database::promote",
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
            } => ShellError::Generic(
                GenericError::new(
                    "Deserializing database failed",
                    "Failed to deserialize database",
                    call_span,
                )
                .with_inner([ShellError::Generic(GenericError::new(
                    "Deserialization failed on a value",
                    error.to_string(),
                    value_span,
                ))]),
            ),

            DatabaseError::PrepareStatement { sql, span, error } => generic_error(
                "nu::shell::database::prepare",
                "Preparing statement failed",
                format!("Error preparing {:?}", sql.as_str()),
                span,
                error,
            ),

            DatabaseError::ExecuteStatement { sql, span, error } => generic_error(
                "nu::shell::database::execute",
                "Executing statement failed",
                format!("Error executing {:?}", sql.as_str()),
                span,
                error,
            ),

            DatabaseError::QueryStatement { sql, span, error } => generic_error(
                "nu::shell::database::query",
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
                "nu::shell::database::iterate",
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
                "nu::shell::database::get",
                "Getting value from database row failed",
                format!("Error at {index:?} for {:?}", sql.as_str()),
                span,
                error,
            ),

            DatabaseError::Unsupported { r#type, span } => generic_error(
                "nu::shell::database::unsupported",
                "Unsupported type for database",
                format!("The type {} is not supported", r#type),
                span,
                None,
            ),

            DatabaseError::CannotConvertIntoDb {
                value,
                expected,
                span,
            } => ShellError::Generic(
                GenericError::new(
                    "Cannot convert value into database",
                    "The input type cannot be converted into a database",
                    span,
                )
                .with_code("nu::shell::database::cannot_convert_into_db")
                .with_inner([ShellError::RuntimeTypeMismatch {
                    expected,
                    actual: value.get_type(),
                    span: value.span(),
                }]),
            ),

            DatabaseError::Todo { msg, span } => generic_error(
                "nu::shell::database::todo",
                "Database To-Do",
                msg,
                span,
                None,
            ),

            DatabaseError::Io(io_error) => ShellError::Io(io_error),

            // TODO: use utf8 error from shell error
            DatabaseError::FromUtf8 { span, error } => generic_error(
                "nu::shell::database::from_utf8",
                "Encountered non-utf8 strings in database",
                error.to_string(),
                span,
                None,
            ),

            DatabaseError::InvalidDeclType {
                rusqlite_type,
                decl_type,
                span,
            } => generic_error(
                "nu::shell::database::invalid_decl_type",
                "Invalid declaration type",
                format!("{} cannot be deserialized as {}", rusqlite_type, decl_type),
                span,
                None,
            ),
        }
    }
}
