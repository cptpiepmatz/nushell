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
        table::DatabaseTable,
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
        let conn = match Connection::open_with_flags(storage.connection_path(), storage.flags()) {
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
            let storage = DatabaseStorage::new_readonly_file(&path, span);
            return Self::open(storage, span);
        }

        let value = pipeline.into_value(span).map_err(DatabaseError::Shell)?;
        Self::open_from_value(value, span)
    }

    pub fn promote(self) -> Result<Self, DatabaseError> {
        if let DatabaseStorage::ReadonlyFile { path, span } = &self.storage {
            let span = *span;
            let storage = DatabaseStorage::new_writable_memory(path.path(), span);
            let mut conn = Self::open(storage, span)?;
            conn.inner
                .restore(DATABASE_NAME, path.path(), None::<fn(Progress)>)
                .map_err(|error| DatabaseError::Promote {
                    path: path.path().into(),
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

    pub fn database_tables(
        &self,
        name: &DatabaseName,
        span: Span,
    ) -> Result<Vec<DatabaseTable>, DatabaseError> {
        let tables_sql = SqlString::new_internal(
            format!("SELECT name FROM {name}.sqlite_master WHERE type='table'"),
            location!(),
        );
        let tables = self.query(tables_sql, DatabaseParams::new_empty(), span)?;

        #[derive(Debug, FromValue)]
        struct TableName {
            name: String,
        }

        Vec::<TableName>::from_value(tables)
            .map_err(DatabaseError::Shell)
            .map(|tables| {
                tables
                    .into_iter()
                    .map(|table| DatabaseTable::UserProvided {
                        name: table.name,
                        span,
                    })
                    .collect()
            })
    }

    pub fn read_table(
        &self,
        name: &DatabaseName,
        table: &DatabaseTable,
        span: Span,
    ) -> Result<Value, DatabaseError> {
        let sql = SqlString::new_internal(format!("SELECT * FROM {name}.{table}"), location!());
        self.query(sql, DatabaseParams::new_empty(), span)
    }

    pub fn read_database(&self, name: &DatabaseName, span: Span) -> Result<Value, DatabaseError> {
        let db_name = name;
        let table_names = self.database_tables(db_name, span)?;

        let mut record = Record::new();
        for table in table_names {
            let values = self.read_table(db_name, &table, span)?;
            record.push(table.to_string(), values);
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
