use std::borrow::Cow;

use nu_protocol::{
    DataSource, FromValue, PipelineData, Record, Span, Spanned, Value, location,
    shell_error::location::Location,
};
use rusqlite::{Connection, backup::Progress};

use crate::database_next::{
    error::DatabaseError,
    plumbing::{
        list::{DatabaseList, DatabaseListEntry},
        name::DatabaseName,
        params::DatabaseParams,
        sql::SqlString,
        statement::DatabaseStatement,
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
                .map_err(|error| DatabaseError::Promote {
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

    pub fn database_list(&self, span: Span) -> Result<DatabaseList, DatabaseError> {
        let sql = SqlString::new_internal("PRAGMA database_list", location!());
        let values = self.query(sql, DatabaseParams::new_empty(), span)?;
        DatabaseList::from_value(values).map_err(DatabaseError::Shell)
    }

    pub fn read_database(&self, name: &DatabaseName, span: Span) -> Result<Value, DatabaseError> {
        let db_name = name;
        let tables_sql = SqlString::new_internal(
            format!("SELECT name FROM {db_name}.sqlite_master WHERE type='table'"),
            location!(),
        );
        let tables = self.query(tables_sql, DatabaseParams::new_empty(), span)?;

        #[derive(Debug, FromValue)]
        struct TableName {
            name: String,
        }

        let table_names = Vec::<TableName>::from_value(tables).map_err(DatabaseError::Shell)?;

        let mut record = Record::new();
        for TableName { name: table_name } in table_names {
            let values_sql = SqlString::new_internal(
                format!("SELECT * FROM {db_name}.{table_name}"),
                location!(),
            );
            let values = self.query(values_sql, DatabaseParams::new_empty(), span)?;
            record.push(table_name, values);
        }

        Ok(Value::record(record, span))
    }

    pub fn read_all(&self, span: Span) -> Result<Value, DatabaseError> {
        let mut record = Record::with_capacity(1); // often only "main"

        let database_list = self.database_list(span)?;
        for DatabaseListEntry { name, .. } in database_list {
            let schema = DatabaseName::new_internal(name.clone(), location!());
            let value = self.read_database(&schema, span)?;
            record.push(name, value);
        }

        return Ok(Value::record(record, span));
    }

    pub fn storage(&self) -> &DatabaseStorage {
        &self.storage
    }
}
