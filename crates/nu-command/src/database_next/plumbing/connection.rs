use nu_protocol::{shell_error::location::Location, Span, Value};
use rusqlite::Connection;

use crate::database_next::{
    error::DatabaseError,
    plumbing::{
        params::DatabaseParams, sql::SqlString, statement::DatabaseStatement,
        storage::DatabaseStorage,
    },
};

#[derive(Debug)]
pub struct DatabaseConnection {
    pub(super) inner: Connection,
}

impl DatabaseConnection {
    pub fn open(storage: impl AsRef<DatabaseStorage>, span: Span) -> Result<Self, DatabaseError> {
        let storage = storage.as_ref();
        let conn =
            Connection::open(storage.as_path()).map_err(|error| DatabaseError::OpenConnection {
                storage: storage.clone(),
                span,
                error,
            })?;
        Ok(Self { inner: conn })
    }

    pub fn open_internal(
        storage: impl AsRef<DatabaseStorage>,
        location: Location,
    ) -> Result<Self, DatabaseError> {
        let _ = (storage, location);
        todo!("implement this as the connection for the history db")
    }

    pub fn prepare(
        &self,
        sql: SqlString,
        span: Span,
    ) -> Result<DatabaseStatement<'_>, DatabaseError> {
        let conn = &self.inner;
        match conn.prepare(sql.as_str()) {
            Ok(stmt) => Ok(DatabaseStatement { inner: stmt, sql }),
            Err(error) => Err(DatabaseError::PrepareStatement { sql, span, error }),
        }
    }

    pub fn execute(
        &self,
        sql: SqlString,
        params: DatabaseParams,
        span: Span,
    ) -> Result<usize, DatabaseError> {
        self.prepare(sql, span)?.execute(params, span)
    }

    pub fn query(
        &self,
        sql: SqlString,
        params: DatabaseParams,
        span: Span,
    ) -> Result<Value, DatabaseError> {
        self.prepare(sql, span)?.query(params, span)
    }
}
