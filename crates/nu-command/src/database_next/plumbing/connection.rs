use nu_protocol::{Span, Value, shell_error::location::Location};
use rusqlite::{Connection, backup::Progress};

use crate::database_next::{
    error::DatabaseError,
    plumbing::{
        params::DatabaseParams, sql::SqlString, statement::DatabaseStatement,
        storage::DatabaseStorage,
    },
};

#[derive(Debug)]
pub struct DatabaseConnection {
    inner: Connection,
    storage: DatabaseStorage,
}

impl DatabaseConnection {
    pub fn open(storage: DatabaseStorage, span: Span) -> Result<Self, DatabaseError> {
        let conn =
            Connection::open(storage.as_path()).map_err(|error| DatabaseError::OpenConnection {
                storage: storage.clone(),
                span,
                error,
            })?;
        Ok(Self {
            inner: conn,
            storage,
        })
    }

    pub fn open_internal(
        storage: impl AsRef<DatabaseStorage>,
        location: Location,
    ) -> Result<Self, DatabaseError> {
        let _ = (storage, location);
        todo!("implement this as the connection for the history db")
    }

    pub fn open_from_value(value: Value) -> Result<Self, DatabaseError> {
        let bytes = value.into_binary().map_err(DatabaseError::Shell)?;
        

        todo!()
    }

    pub fn promote(self) -> Result<Self, DatabaseError> {
        if let DatabaseStorage::ReadonlyFile { path, span } = &self.storage {
            let span = *span;
            let storage = DatabaseStorage::new_writable_memory(path, span);
            let mut conn = Self::open(storage, span)?;
            conn.inner
                .restore("main", path, None::<fn(Progress)>)
                .map_err(|error| DatabaseError::Restore {
                    path: path.into(),
                    span,
                    error,
                })?;
            return Ok(conn);
        }

        Ok(self)
    }

    pub fn prepare(
        &self,
        sql: SqlString,
        span: Span,
    ) -> Result<DatabaseStatement<'_>, DatabaseError> {
        let conn = &self.inner;
        match conn.prepare(sql.as_str()) {
            Ok(stmt) => Ok(DatabaseStatement::new(stmt, sql)),
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
