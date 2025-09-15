use nu_protocol::{
    DataSource, FromValue, PipelineData, Span, Spanned, Value, shell_error::location::Location,
};
use rusqlite::{Connection, backup::Progress};

use crate::database_next::{
    error::DatabaseError,
    plumbing::{
        params::DatabaseParams, sql::SqlString, statement::DatabaseStatement,
        storage::DatabaseStorage,
    },
};

/// Name for the used database.
///
/// In a typical sqlite setup with a connection only keeping one database open, you only have "main".
const DATABASE_NAME: &str = "main";

#[derive(Debug)]
pub struct DatabaseConnection {
    inner: Connection,
    storage: DatabaseStorage,
}

impl DatabaseConnection {
    fn open_raw(storage: DatabaseStorage) -> Result<Self, (rusqlite::Error, DatabaseStorage)> {
        let conn = match Connection::open_with_flags(storage.as_path(), storage.flags()) {
            Ok(conn) => conn,
            Err(err) => return Err((err, storage)),
        };

        Ok(Self {
            inner: conn,
            storage,
        })
    }

    pub fn open(storage: DatabaseStorage, span: Span) -> Result<Self, DatabaseError> {
        Self::open_raw(storage).map_err(|(error, storage)| DatabaseError::OpenConnection {
            storage,
            span,
            error,
        })
    }

    pub fn open_internal(
        storage: DatabaseStorage,
        location: Location,
    ) -> Result<Self, DatabaseError> {
        Self::open_raw(storage).map_err(|(error, storage)| DatabaseError::OpenInternalConnection {
            storage,
            location,
            error,
        })
    }

    pub fn open_from_value(value: Value, span: Span) -> Result<Self, DatabaseError> {
        let bytes = Spanned::<Vec<u8>>::from_value(value).map_err(DatabaseError::Shell)?;
        let storage = DatabaseStorage::new_writable_memory(&bytes.item, span);
        let mut conn = Self::open(storage, span)?;
        conn.inner
            .deserialize_read_exact(
                DATABASE_NAME,
                bytes.item.as_slice(),
                bytes.item.len(),
                false,
            )
            .map_err(|error| DatabaseError::Deserialize {
                call_span: span,
                value_span: bytes.span,
                error,
            })?;
        Ok(conn)
    }

    pub fn open_from_pipeline(pipeline: PipelineData, span: Span) -> Result<Self, DatabaseError> {
        if let Some(metadata) = pipeline.metadata()
            && let DataSource::FilePath(path) = metadata.data_source
        {
            let path = nu_path::PathBuf::from(path)
                .try_into_absolute()
                .map_err(|_| DatabaseError::Todo {
                    msg: "Handle non absolute paths from pipeline".into(),
                    span,
                })?;
            let storage = DatabaseStorage::ReadonlyFile { path, span };
            return Self::open(storage, span);
        }

        let value = pipeline.into_value(span).map_err(DatabaseError::Shell)?;
        Self::open_from_value(value, span)
    }

    pub fn promote(self) -> Result<Self, DatabaseError> {
        if let DatabaseStorage::ReadonlyFile { path, span } = &self.storage {
            let span = *span;
            let storage = DatabaseStorage::new_writable_memory(path, span);
            let mut conn = Self::open(storage, span)?;
            conn.inner
                .restore(DATABASE_NAME, path, None::<fn(Progress)>)
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
