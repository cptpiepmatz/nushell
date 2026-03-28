use std::iter;

use itertools::Itertools;
use nu_protocol::{DataSource, FromValue, PipelineData, Record, Span, Spanned, Type, Value};
use nu_utils::{location::Location, push_fmt};
use rusqlite::{Connection, backup::Progress};

use crate::database_nova::{
    error::DatabaseError,
    plumbing::{
        column::DatabaseColumn,
        decl_type::DatabaseDeclType,
        list::{DatabaseList, DatabaseListEntry},
        name::DatabaseName,
        params::DatabaseParams,
        sql::SqlString,
        statement::DatabaseStatement,
        storage::DatabaseStorage,
        table::DatabaseTableName,
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

    #[track_caller]
    pub fn open_internal(storage: DatabaseStorage) -> Result<Self, DatabaseError> {
        let location = Location::caller();
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
        if let Some(metadata) = pipeline.metadata_ref()
            && let DataSource::FilePath(path) = &metadata.data_source
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

    pub fn new_empty(span: Span) -> Result<Self, DatabaseError> {
        let storage = DatabaseStorage::new_writable_memory_counted(span);
        Self::open(storage, span)
    }

    pub fn new_from_value<'t>(
        value: Value,
        table_name: impl Into<DatabaseTableName>,
        span: Span,
    ) -> Result<Self, DatabaseError> {
        let strict = false;

        let ty = value.get_type();
        let types = match ty {
            Type::Table(types) => types,
            ty => {
                return Err(DatabaseError::CannotConvertIntoDb {
                    value,
                    expected: Type::table(),
                    span,
                });
            }
        };

        let records = value
            .into_list()
            .map_err(DatabaseError::Shell)?
            .into_iter()
            .map(|v| v.into_record())
            .collect::<Result<Vec<_>, _>>()
            .map_err(DatabaseError::Shell)?;

        let columns = types
            .into_iter()
            .map(|(name, ty)| {
                Ok(DatabaseColumn {
                    name,
                    decl_type: Some(DatabaseDeclType::try_from_type(&ty, span)?),
                })
            })
            .collect::<Result<Vec<_>, DatabaseError>>()?;
        let column_iter = || {
            columns
                .iter()
                .zip(columns.iter().skip(1).map(Some).chain(iter::repeat(None)))
        };

        let connection = Self::new_empty(span)?;

        // TODO: use transaction, not necessary but feels like best practice here

        let table_name = table_name.into();
        let sql = {
            let mut sql = String::new();
            push_fmt!(sql, "CREATE TABLE {} (\n", table_name.sql_name());
            for (col, next) in column_iter() {
                push_fmt!(
                    sql,
                    "    {} {}",
                    col.sql_name(),
                    col.decl_type.expect("is some").as_str(strict)
                );
                next.into_iter().for_each(|_| push_fmt!(sql, ","));
                push_fmt!(sql, "\n");
            }
            push_fmt!(sql, ")");
            SqlString::new_internal(sql)
        };
        connection.execute(sql, DatabaseParams::new_empty(), span)?;

        let insert_sql = {
            let mut sql = String::new();
            push_fmt!(sql, "INSERT INTO {} (", table_name.sql_name());
            for (col, next) in column_iter() {
                push_fmt!(sql, "{}", col.sql_name());
                next.into_iter().for_each(|_| push_fmt!(sql, ", "));
            }
            push_fmt!(sql, ")\n");
            push_fmt!(sql, "VALUES (");
            for (col, next) in column_iter() {
                push_fmt!(sql, "?");
                next.into_iter().for_each(|_| push_fmt!(sql, ", "));
            }
            push_fmt!(sql, ")");
            SqlString::new_internal(sql)
        };
        let mut insert_stmt = connection.prepare(insert_sql, span)?;

        for mut record in records {
            let values = columns.iter().map(|col| {
                record
                    .remove(&col.name)
                    .unwrap_or(Value::nothing(Span::unknown()))
            });
            let params = DatabaseParams::new_unnamed(values)?;
            insert_stmt.execute(params, span)?;
        }

        drop(insert_stmt);
        Ok(connection)
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
        let sql = SqlString::new_internal("PRAGMA database_list");
        let values = self.query(sql, DatabaseParams::new_empty(), span)?;
        DatabaseList::from_value(values).map_err(DatabaseError::Shell)
    }

    pub fn database_tables(
        &self,
        name: &DatabaseName,
        span: Span,
    ) -> Result<Vec<DatabaseTableName>, DatabaseError> {
        let tables_sql = SqlString::new_internal(format!(
            "SELECT name FROM {name}.sqlite_master WHERE type='table'"
        ));
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
                    .map(|table| DatabaseTableName::UserProvided {
                        name: table.name,
                        span,
                    })
                    .collect()
            })
    }

    pub fn read_table(
        &self,
        name: &DatabaseName,
        table: &DatabaseTableName,
        span: Span,
    ) -> Result<Value, DatabaseError> {
        let sql = SqlString::new_internal(format!("SELECT * FROM {name}.{table}"));
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
            let schema = DatabaseName::new_internal(name.clone());
            let value = self.read_database(&schema, span)?;
            record.push(name, value);
        }

        Ok(Value::record(record, span))
    }

    pub fn storage(&self) -> &DatabaseStorage {
        &self.storage
    }
}
