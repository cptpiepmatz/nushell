use crate::database::{error::DatabaseError, values::dto::ValueDto};

use super::definitions::{
    db_column::DbColumn, db_constraint::DbConstraint, db_foreignkey::DbForeignKey,
    db_index::DbIndex, db_table::DbTable,
};
use nu_protocol::{
    CustomValue, FromValue, IntoValue, PipelineData, Record, ShellError, Signals, Span, Spanned,
    Type, Value, engine::EngineState, shell_error::io::IoError,
};
use rusqlite::{
    Connection, DatabaseName, Error as SqliteError, Row, RowIndex, Statement, ToSql,
    params_from_iter,
    types::{FromSql, ValueRef},
};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    fmt::{Debug, Display},
    fs::File,
    io::Read,
    ops::Deref,
    path::{Path, PathBuf},
    str::FromStr,
};

const SQLITE_MAGIC_BYTES: &[u8; 16] = b"SQLite format 3\0";
const MEMORY_DB: &str = "file:memdb1?mode=memory&cache=shared";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SQLiteDatabase {
    /// Path representation to build [`Connection`]s.
    ///
    /// # Implementation Notes
    /// This doesn't store a `Connection` directly because:
    /// - YAGNI
    /// - not obvious how cloning could work
    /// - tricky state management
    pub path: DatabasePath,

    // Skip serialization for this as `CustomValue`s only really get serialized for plugins.
    #[serde(skip, default = "Signals::empty")]
    signals: Signals,
}

/// Unspanned methods.
///
/// All of these methods do not work with spans as they operate without any user input.
/// Therefore do all of these return [`DatabaseError`] which can be converted into a [`ShellError`]
/// using [`DatabaseError::into_shell_error`] with a provided [`Span`].
impl SQLiteDatabase {
    const TYPE_NAME: &str = "SQLiteDatabase";

    /// Construct a new `SQLiteDatabase` to be stored on disk.
    pub fn new(path: impl Into<PathBuf>, signals: Signals) -> Self {
        Self {
            path: DatabasePath::Path(path.into()),
            signals,
        }
    }

    /// Construct a new in-memory `SQLiteDatabase`.
    pub fn new_in_memory(signals: Signals) -> Self {
        Self {
            path: DatabasePath::InMemory,
            signals,
        }
    }

    /// Construct a new in-memory `SQLiteDatabase` with custom parameters.
    pub fn new_in_custom_memory(signals: Signals) -> Self {
        Self {
            path: DatabasePath::InMemoryCustom,
            signals,
        }
    }

    pub fn read_from_path(
        path: impl Into<PathBuf>,
        signals: Signals,
        span: Span,
    ) -> Result<Self, DatabaseError> {
        let path = path.into();
        let mut file =
            File::open(&path).map_err(|error| IoError::new(error, span, path.clone()))?;

        let mut buf: [u8; 16] = [0; 16];
        file.read_exact(&mut buf).map_err(|error| {
            IoError::new_with_additional_context(
                error,
                span,
                path.clone(),
                "Could not read magic bytes for SQLite database file",
            )
        })?;

        match &buf == SQLITE_MAGIC_BYTES {
            true => Ok(SQLiteDatabase::new(path, signals)),
            false => Err(DatabaseError::NotASqliteFile { path }),
        }
    }

    pub fn try_from_pipeline(input: PipelineData, span: Span) -> Result<Self, DatabaseError> {
        Self::from_value(input.into_value(span).map_err(DatabaseError::Shell)?)
            .map_err(DatabaseError::Shell)
    }

    pub fn open_connection(&self) -> Result<Connection, DatabaseError> {
        let (conn, set_busy_handler) = match self.path {
            DatabasePath::Path(path_buf) => (Connection::open(&path_buf), true),
            DatabasePath::InMemory => (Connection::open_in_memory(), false),
            DatabasePath::InMemoryCustom => (Connection::open(MEMORY_DB), true),
        };

        let conn = conn.map_err(|error| DatabaseError::OpenConnection {
            path: self.path.clone(),
            error,
        })?;

        if set_busy_handler {
            conn.busy_handler(Some(SQLiteDatabase::sleeper))
                .map_err(|error| DatabaseError::SetBusyHandler {
                    path: self.path.clone(),
                    error,
                })?;
        }

        Ok(conn)
    }

    fn sleeper(attempts: i32) -> bool {
        log::warn!("SQLITE_BUSY, retrying after 250ms (attempt {attempts})");
        std::thread::sleep(std::time::Duration::from_millis(250));
        true
    }

    pub fn get_tables(&self, conn: &Connection) -> Result<Vec<DbTable>, DatabaseError> {
        let table_names_sql = "SELECT name FROM sqlite_master WHERE type = 'table'";
        let mut table_names =
            conn.prepare(table_names_sql)
                .map_err(|error| DatabaseError::Prepare {
                    sql: table_names_sql.into(),
                    error,
                })?;

        let rows = table_names
            .query_map([], |row| row.get(0))
            .map_err(|error| DatabaseError::Query {
                sql: table_names_sql.into(),
                error,
            })?;
        let mut tables = Vec::new();

        for (idx, row) in rows.enumerate() {
            let table_name: String = row.map_err(|error| DatabaseError::Iterate {
                sql: table_names_sql.into(),
                index: idx,
                error,
            })?;
            tables.push(DbTable {
                name: table_name,
                create_time: None,
                update_time: None,
                engine: None,
                schema: None,
            })
        }

        Ok(tables.into_iter().collect())
    }

    pub fn drop_all_tables(&self, conn: &Connection) -> Result<(), DatabaseError> {
        let tables = self.get_tables(conn)?;

        for table in tables {
            let sql = format!("DROP TABLE {}", table.name);
            conn.execute(&sql, [])
                .map_err(|error| DatabaseError::Execute {
                    sql: sql.into(),
                    error,
                })?;
        }

        Ok(())
    }

    pub fn export_in_memory_database_to_file(
        &self,
        conn: &Connection,
        filename: String,
    ) -> Result<(), DatabaseError> {
        //vacuum main into 'c:\\temp\\foo.db'
        let sql = format!("vacuum main into '{filename}'");
        conn.execute(&sql, [])
            .map_err(|error| DatabaseError::Execute {
                sql: sql.into(),
                error,
            })?;

        Ok(())
    }

    pub fn backup_database_to_file(
        &self,
        conn: &Connection,
        filename: String,
    ) -> Result<(), DatabaseError> {
        let filename = PathBuf::from(filename);
        conn.backup(DatabaseName::Main, &filename, None)
            .map_err(|error| DatabaseError::Backup {
                database_name: DatabaseName::Main,
                path: filename.into(),
                error,
            })?;
        Ok(())
    }

    pub fn restore_database_from_file(
        &self,
        conn: &mut Connection,
        filename: String,
    ) -> Result<(), DatabaseError> {
        let filename = PathBuf::from(filename);
        conn.restore(
            DatabaseName::Main,
            &filename,
            Some(|p: rusqlite::backup::Progress| {
                let percent = if p.pagecount == 0 {
                    100
                } else {
                    (p.pagecount - p.remaining) * 100 / p.pagecount
                };
                if percent % 10 == 0 {
                    log::trace!("Restoring: {percent} %");
                }
            }),
        )
        .map_err(|error| DatabaseError::Restore {
            database_name: DatabaseName::Main,
            path: filename.into(),
            error,
        })?;
        Ok(())
    }

    fn get_column_info(row: &Row) -> Result<DbColumn, DatabaseError> {
        Ok(DbColumn {
            cid: Self::get_column("cid", row)?,
            name: Self::get_column("name", row)?,
            r#type: Self::get_column("type", row)?,
            notnull: Self::get_column("notnull", row)?,
            default: Self::get_column("dflt_value", row)?,
            pk: Self::get_column("pk", row)?,
        })
    }

    pub fn get_columns(
        &self,
        conn: &Connection,
        table: &DbTable,
    ) -> Result<Vec<DbColumn>, DatabaseError> {
        let sql = format!("SELECT * FROM pragma_table_info('{}');", table.name);
        Self::query_infos(conn, sql.into(), Self::get_column_info)
    }

    fn get_constraint_info(row: &Row) -> Result<DbConstraint, DatabaseError> {
        Ok(DbConstraint {
            name: Self::get_column("index_name", row)?,
            column_name: Self::get_column("column_name", row)?,
            origin: Self::get_column("origin", row)?,
        })
    }

    pub fn get_constraints(
        &self,
        conn: &Connection,
        table: &DbTable,
    ) -> Result<Vec<DbConstraint>, DatabaseError> {
        let sql = format!(
            "
            SELECT
                p.origin,
                s.name AS index_name,
                i.name AS column_name
            FROM
                sqlite_master s
                JOIN pragma_index_list(s.tbl_name) p ON s.name = p.name,
                pragma_index_info(s.name) i
            WHERE
                s.type = 'index'
                AND tbl_name = '{}'
                AND NOT p.origin = 'c'
            ",
            &table.name
        );

        Self::query_infos(conn, sql.into(), Self::get_constraint_info)
    }

    fn get_foreign_key_info(row: &Row) -> Result<DbForeignKey, DatabaseError> {
        Ok(DbForeignKey {
            column_name: Self::get_column("from", row)?,
            ref_table: Self::get_column("table", row)?,
            ref_column: Self::get_column("to", row)?,
        })
    }

    pub fn get_foreign_keys(
        &self,
        conn: &Connection,
        table: &DbTable,
    ) -> Result<Vec<DbForeignKey>, DatabaseError> {
        let sql = format!(
            "SELECT p.`from`, p.`to`, p.`table` FROM pragma_foreign_key_list('{}') p",
            &table.name
        );

        Self::query_infos(conn, sql.into(), Self::get_foreign_key_info)
    }

    fn get_index_info(row: &Row) -> Result<DbIndex, DatabaseError> {
        Ok(DbIndex {
            name: Self::get_column("index_name", row)?,
            column_name: Self::get_column("name", row)?,
            seqno: Self::get_column("seqno", row)?,
        })
    }

    pub fn get_indexes(
        &self,
        conn: &Connection,
        table: &DbTable,
    ) -> Result<Vec<DbIndex>, DatabaseError> {
        let sql = format!(
            "
            SELECT
                m.name AS index_name,
                p.*
            FROM
                sqlite_master m,
                pragma_index_info(m.name) p
            WHERE
                m.type = 'index'
                AND m.tbl_name = '{}'
            ",
            &table.name,
        );

        Self::query_infos(conn, sql.into(), Self::get_index_info)
    }

    fn get_column<T: FromSql>(
        idx: impl RowIndex + Debug + Copy + 'static,
        row: &Row,
    ) -> Result<T, DatabaseError> {
        row.get(idx).map_err(|error| DatabaseError::Get {
            sql: row.as_ref().expanded_sql().map(Cow::Owned),
            index: Box::new(idx),
            error,
        })
    }

    fn query_infos<T>(
        conn: &Connection,
        sql: Cow<'static, str>,
        read_query: impl for<'r> Fn(&'r Row<'r>) -> Result<T, DatabaseError>,
    ) -> Result<Vec<T>, DatabaseError> {
        let mut column_names = match conn.prepare(&sql) {
            Ok(column_names) => column_names,
            Err(error) => return Err(DatabaseError::Prepare { sql, error }),
        };

        let mut infos: Vec<T> = Vec::new();
        let mut rows = match column_names.query([]) {
            Ok(rows) => rows,
            Err(error) => return Err(DatabaseError::Query { sql, error }),
        };

        for i in 0.. {
            match rows.next() {
                Ok(None) => break,
                Ok(Some(row)) => infos.push(read_query(row)?),
                Err(error) => {
                    return Err(DatabaseError::Iterate {
                        sql,
                        index: i,
                        error,
                    });
                }
            }
        }

        Ok(infos)
    }
}

/// Spanned methods.
///
/// These methods are expected to be used from user input and therefore all work with spans.
/// All of these include a [`call_span`](Span) to allow providing spanned errors.
/// These methods directly return [`ShellError`] and need no further span handling.
impl SQLiteDatabase {
    pub fn query<'c>(
        &self,
        conn: impl Into<Option<&'c Connection>>,
        sql: impl Into<SqlInput>,
        params: impl Into<Option<NuSqlParams>>,
        call_span: Span,
    ) -> Result<Value, ShellError> {
        let sql = sql.into();

        let conn = match conn.into() {
            Some(conn) => conn,
            None => &self
                .open_connection()
                .map_err(|error| error.into_shell_error(call_span))?,
        };

        let mut stmt = conn.prepare(sql.as_str()).map_err(|error| {
            DatabaseError::Prepare {
                sql: sql.clone(),
                error,
            }
            .into_shell_error(call_span)
        })?;

        let columns: Vec<TypedColumn> = stmt.columns().iter().map(TypedColumn::from).collect();

        let params = params.into().unwrap_or_default();
        let rows = match params {
            NuSqlParams::List(items) => {
                stmt.query(params_from_iter(items.iter().map(|item| item.deref())))
            }
            NuSqlParams::Named(items) => {
                let params: Vec<(&str, &dyn ToSql)> = items
                    .iter()
                    .map(|(key, val)| (key.as_str(), val.deref()))
                    .collect();
                stmt.query(params.as_slice())
            }
        };
        let mut rows = rows.map_err(|error| {
            DatabaseError::Query {
                sql: sql.clone(),
                error,
            }
            .into_shell_error(call_span)
        })?;

        let mut row_values = Vec::new();
        for idx in 0.. {
            self.signals.check(&call_span)?;
            match rows.next() {
                Ok(None) => break,
                Ok(Some(row)) => row_values.push(self.row_to_value(row, call_span, &columns)),
                Err(error) => {
                    return Err(DatabaseError::Iterate {
                        sql: sql.clone(),
                        index: idx,
                        error,
                    }
                    .into_shell_error(call_span));
                }
            }
        }

        Ok(Value::list(row_values, call_span))
    }

    fn row_to_value(&self, row: &Row, span: Span, columns: &[TypedColumn]) -> Value {
        Value::record(
            Record::from_iter(columns.iter().enumerate().map(|(i, col)| {
                (
                    col.name.clone(),
                    ValueDto::from_value_ref(row.get_ref_unwrap(i), col.decl_type, span).0,
                )
            })),
            span,
        )
    }

    pub fn read_all<'c>(
        &self,
        conn: impl Into<Option<&'c Connection>>,
        call_span: Span,
    ) -> Result<Value, ShellError> {
        let conn = match conn.into() {
            Some(conn) => conn,
            None => &self
                .open_connection()
                .map_err(|err| err.into_shell_error(call_span))?,
        };

        let get_table_names_sql = "SELECT name FROM sqlite_master WHERE type = 'table'";
        let mut get_table_names = conn.prepare(get_table_names_sql).map_err(|error| {
            DatabaseError::Prepare {
                sql: get_table_names_sql.into(),
                error,
            }
            .into_shell_error(call_span)
        })?;

        let mut rows = get_table_names.query([]).map_err(|error| {
            DatabaseError::Query {
                sql: get_table_names_sql.into(),
                error,
            }
            .into_shell_error(call_span)
        })?;

        let mut tables = Record::new();
        for i in 0.. {
            self.signals.check(&call_span)?;
            match rows.next() {
                Err(error) => {
                    return Err(DatabaseError::Iterate {
                        sql: get_table_names_sql.into(),
                        index: i,
                        error,
                    }
                    .into_shell_error(call_span));
                }
                Ok(None) => break,
                Ok(Some(row)) => {
                    let table_name: String =
                        Self::get_column(0, row).map_err(|err| err.into_shell_error(call_span))?;
                    let table_sql = format!("SELECT * FROM [{table_name}]");
                    let rows = self.query(conn, table_sql, None, call_span)?;
                    tables.push(table_name, rows);
                }
            }
        }

        Ok(Value::record(tables, call_span))
    }

    pub fn read_one<'c>(
        &self,
        conn: impl Into<Option<&'c Connection>>,
        table_name: Spanned<String>,
        call_span: Span,
    ) -> Result<Value, ShellError> {
        let conn = match conn.into() {
            Some(conn) => conn,
            None => &self
                .open_connection()
                .map_err(|err| err.into_shell_error(call_span))?,
        };

        let sql = SqlInput::Spanned { value: format!("SELECT * FROM [{}]", table_name.item), span: table_name.span };

        self.query(conn, sql, None, call_span)
    }

    pub fn build_params(&self, value: Value) -> Result<NuSqlParams, ShellError> {
        match value {
            Value::Record { val, .. } => Ok(NuSqlParams::Named(
                val.into_owned()
                    .into_iter()
                    .map(|(mut column, value)| {
                        if !column.starts_with([':', '@', '$']) {
                            column.insert(0, ':')
                        };
                        (column, Box::new(ValueDto(value)) as Box<dyn ToSql>)
                    })
                    .collect(),
            )),

            Value::List { vals, .. } => Ok(NuSqlParams::List(
                vals.into_iter()
                    .map(|val| Box::new(ValueDto(val)) as Box<dyn ToSql>)
                    .collect(),
            )),

            // We accept no parameters
            Value::Nothing { .. } => Ok(NuSqlParams::default()),

            _ => Err(ShellError::TypeMismatch {
                err_message: "Invalid parameters value: expected record or list".to_string(),
                span: value.span(),
            }),
        }
    }
}

impl FromValue for SQLiteDatabase {
    fn from_value(value: Value) -> Result<Self, ShellError> {
        let span = value.span();
        match value {
            Value::Custom { val, .. } => match val.as_any().downcast_ref::<Self>() {
                Some(db) => Ok(Self {
                    path: db.path.clone(),
                    signals: db.signals.clone(),
                }),
                None => Err(ShellError::CantConvert {
                    to_type: Self::expected_type().to_string(),
                    from_type: val.type_name(),
                    span,
                    help: None,
                }),
            },
            x => Err(ShellError::CantConvert {
                to_type: Self::expected_type().to_string(),
                from_type: x.get_type().to_string(),
                span: x.span(),
                help: None,
            }),
        }
    }

    fn expected_type() -> Type {
        Type::custom(Self::TYPE_NAME)
    }
}

impl IntoValue for SQLiteDatabase {
    fn into_value(self, span: Span) -> Value {
        Value::custom(Box::new(self), span)
    }
}

impl CustomValue for SQLiteDatabase {
    fn clone_value(&self, span: Span) -> Value {
        Value::custom(Box::new(self.clone()), span)
    }

    fn type_name(&self) -> String {
        self.typetag_name().to_string()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        let db = self
            .open_connection()
            .map_err(|err| err.into_shell_error(span))?;
        read_entire_sqlite_db(db, span, &self.signals)
            .map_err(|e| e.into_shell_error(span, "Failed to read from SQLite database"))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn follow_path_int(
        &self,
        _self_span: Span,
        _index: usize,
        path_span: Span,
    ) -> Result<Value, ShellError> {
        // In theory we could support this, but tables don't have an especially well-defined order
        Err(ShellError::IncompatiblePathAccess { type_name: "SQLite databases do not support integer-indexed access. Try specifying a table name instead".into(), span: path_span })
    }

    fn follow_path_string(
        &self,
        _self_span: Span,
        column_name: String,
        path_span: Span,
    ) -> Result<Value, ShellError> {
        let db = open_sqlite_db(&self.path, path_span)?;
        read_single_table(db, column_name, path_span, &self.signals)
            .map_err(|e| e.into_shell_error(path_span, "Failed to read from SQLite database"))
    }

    fn typetag_name(&self) -> &'static str {
        Self::TYPE_NAME
    }

    fn typetag_deserialize(&self) {
        unimplemented!("typetag_deserialize")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabasePath {
    /// Store database on disk.
    Path(Spanned<PathBuf>),

    /// Store database in memory.
    InMemory,

    /// Store database in memory with custom parameters.
    InMemoryCustom,
}

impl DatabasePath {
    /// [`Path`] representation of a `DatabasePath`.
    ///
    /// Returns [`None`] for `DatabasePath::Memory`.
    pub fn as_path(&self) -> Option<&Path> {
        match self {
            DatabasePath::Path(path_buf) => Some(path_buf.item.as_path()),
            DatabasePath::InMemory => None,
            DatabasePath::InMemoryCustom => Some(Path::new(MEMORY_DB)),
        }
    }
}

#[derive(Clone, Debug)]
pub enum SqlInput {
    Static(&'static str),
    Owned(String),
    Spanned { value: String, span: Span },
}

impl SqlInput {
    pub fn as_str(&self) -> &str {
        match self {
            SqlInput::Static(s) => s,
            SqlInput::Owned(s) => s,
            SqlInput::Spanned { value: s, .. } => s,
        }
    }

    pub fn span(&self) -> Option<Span> {
        match self {
            Self::Spanned { span, .. } => Some(*span),
            _ => None,
        }
    }
}

impl From<&'static str> for SqlInput {
    fn from(value: &'static str) -> Self {
        Self::Static(value)
    }
}

impl From<String> for SqlInput {
    fn from(value: String) -> Self {
        Self::Owned(value)
    }
}

impl From<Spanned<String>> for SqlInput {
    fn from(value: Spanned<String>) -> Self {
        Self::Spanned {
            value: value.item,
            span: value.span,
        }
    }
}

impl Display for SqlInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SqlInput::Static(s) => Display::fmt(s, f),
            SqlInput::Owned(s) => Display::fmt(s, f),
            SqlInput::Spanned { value: s, .. } => Display::fmt(s, f),
        }
    }
}

#[deprecated]
pub fn open_sqlite_db(path: &Path, call_span: Span) -> Result<Connection, ShellError> {
    if path.to_string_lossy() == MEMORY_DB {
        open_connection_in_memory_custom()
    } else {
        let path = path.to_string_lossy().to_string();
        Connection::open(path).map_err(|err| ShellError::GenericError {
            error: "Failed to open SQLite database".into(),
            msg: err.to_string(),
            span: Some(call_span),
            help: None,
            inner: Vec::new(),
        })
    }
}

#[deprecated]
fn run_sql_query(
    conn: Connection,
    sql: &Spanned<String>,
    params: NuSqlParams,
    signals: &Signals,
) -> Result<Value, SqliteOrShellError> {
    let stmt = conn.prepare(&sql.item)?;
    prepared_statement_to_nu_list(stmt, params, sql.span, signals)
}

#[deprecated]
// This is taken from to text local_into_string but tweaks it a bit so that certain formatting does not happen
pub fn value_to_sql(
    engine_state: &EngineState,
    value: Value,
    call_span: Span,
) -> Result<Box<dyn rusqlite::ToSql>, ShellError> {
    match value {
        Value::Bool { val, .. } => Ok(Box::new(val)),
        Value::Int { val, .. } => Ok(Box::new(val)),
        Value::Float { val, .. } => Ok(Box::new(val)),
        Value::Filesize { val, .. } => Ok(Box::new(val.get())),
        Value::Duration { val, .. } => Ok(Box::new(val)),
        Value::Date { val, .. } => Ok(Box::new(val)),
        Value::String { val, .. } => Ok(Box::new(val)),
        Value::Binary { val, .. } => Ok(Box::new(val)),
        Value::Nothing { .. } => Ok(Box::new(rusqlite::types::Null)),
        val => {
            let json_value = crate::value_to_json_value(engine_state, &val, call_span, false)?;
            match nu_json::to_string_raw(&json_value) {
                Ok(s) => Ok(Box::new(s)),
                Err(err) => Err(ShellError::CantConvert {
                    to_type: "JSON".into(),
                    from_type: val.get_type().to_string(),
                    span: val.span(),
                    help: Some(err.to_string()),
                }),
            }
        }
    }
}

#[deprecated]
pub fn values_to_sql(
    engine_state: &EngineState,
    values: impl IntoIterator<Item = Value>,
    call_span: Span,
) -> Result<Vec<Box<dyn rusqlite::ToSql>>, ShellError> {
    values
        .into_iter()
        .map(|v| value_to_sql(engine_state, v, call_span))
        .collect::<Result<Vec<_>, _>>()
}

pub enum NuSqlParams {
    List(Vec<Box<dyn ToSql>>),
    Named(Vec<(String, Box<dyn ToSql>)>),
}

impl Default for NuSqlParams {
    fn default() -> Self {
        NuSqlParams::List(Vec::new())
    }
}

#[deprecated]
pub fn nu_value_to_params(
    engine_state: &EngineState,
    value: Value,
    call_span: Span,
) -> Result<NuSqlParams, ShellError> {
    match value {
        Value::Record { val, .. } => {
            let mut params = Vec::with_capacity(val.len());

            for (mut column, value) in val.into_owned().into_iter() {
                let sql_type_erased = value_to_sql(engine_state, value, call_span)?;

                if !column.starts_with([':', '@', '$']) {
                    column.insert(0, ':');
                }

                params.push((column, sql_type_erased));
            }

            Ok(NuSqlParams::Named(params))
        }
        Value::List { vals, .. } => {
            let mut params = Vec::with_capacity(vals.len());

            for value in vals.into_iter() {
                let sql_type_erased = value_to_sql(engine_state, value, call_span)?;

                params.push(sql_type_erased);
            }

            Ok(NuSqlParams::List(params))
        }

        // We accept no parameters
        Value::Nothing { .. } => Ok(NuSqlParams::default()),

        _ => Err(ShellError::TypeMismatch {
            err_message: "Invalid parameters value: expected record or list".to_string(),
            span: value.span(),
        }),
    }
}

#[deprecated]
#[derive(Debug)]
enum SqliteOrShellError {
    SqliteError(SqliteError),
    ShellError(ShellError),
}

impl From<SqliteError> for SqliteOrShellError {
    fn from(error: SqliteError) -> Self {
        Self::SqliteError(error)
    }
}

impl From<ShellError> for SqliteOrShellError {
    fn from(error: ShellError) -> Self {
        Self::ShellError(error)
    }
}

impl SqliteOrShellError {
    fn into_shell_error(self, span: Span, msg: &str) -> ShellError {
        match self {
            Self::SqliteError(err) => ShellError::GenericError {
                error: msg.into(),
                msg: err.to_string(),
                span: Some(span),
                help: None,
                inner: Vec::new(),
            },
            Self::ShellError(err) => err,
        }
    }
}

#[deprecated]
fn read_single_table(
    conn: Connection,
    table_name: String,
    call_span: Span,
    signals: &Signals,
) -> Result<Value, SqliteOrShellError> {
    // TODO: Should use params here?
    let stmt = conn.prepare(&format!("SELECT * FROM [{table_name}]"))?;
    prepared_statement_to_nu_list(stmt, NuSqlParams::default(), call_span, signals)
}

/// The SQLite type behind a query column returned as some raw type (e.g. 'text')
#[derive(Clone, Copy)]
pub enum DeclType {
    Json,
    Jsonb,
}

impl DeclType {
    #[deprecated]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "JSON" => Some(DeclType::Json),
            "JSONB" => Some(DeclType::Jsonb),
            _ => None, // We are only special-casing JSON(B) columns for now
        }
    }
}

impl FromStr for DeclType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "JSON" => Ok(DeclType::Json),
            "JSONB" => Ok(DeclType::Jsonb),
            _ => Err(()), // We are only special-casing JSON(B) columns for now
        }
    }
}

/// A column out of an SQLite query, together with its type
pub struct TypedColumn {
    pub name: String,
    pub decl_type: Option<DeclType>,
}

impl<'s> From<&rusqlite::Column<'s>> for TypedColumn {
    fn from(c: &rusqlite::Column<'s>) -> Self {
        Self {
            name: c.name().to_owned(),
            decl_type: c.decl_type().and_then(DeclType::from_str),
        }
    }
}

impl TypedColumn {
    #[deprecated]
    pub fn from_rusqlite_column(c: &rusqlite::Column) -> Self {
        Self {
            name: c.name().to_owned(),
            decl_type: c.decl_type().and_then(DeclType::from_str),
        }
    }
}

#[deprecated]
fn prepared_statement_to_nu_list(
    mut stmt: Statement,
    params: NuSqlParams,
    call_span: Span,
    signals: &Signals,
) -> Result<Value, SqliteOrShellError> {
    let columns: Vec<TypedColumn> = stmt
        .columns()
        .iter()
        .map(TypedColumn::from_rusqlite_column)
        .collect();

    // I'm very sorry for this repetition
    // I tried scoping the match arms to the query_map alone, but lifetime and closure reference escapes
    // got heavily in the way
    let row_values = match params {
        NuSqlParams::List(params) => {
            let refs: Vec<&dyn ToSql> = params.iter().map(|value| &**value).collect();

            let row_results = stmt.query_map(refs.as_slice(), |row| {
                Ok(convert_sqlite_row_to_nu_value(row, call_span, &columns))
            })?;

            // we collect all rows before returning them. Not ideal but it's hard/impossible to return a stream from a CustomValue
            let mut row_values = vec![];

            for row_result in row_results {
                signals.check(&call_span)?;
                if let Ok(row_value) = row_result {
                    row_values.push(row_value);
                }
            }

            row_values
        }
        NuSqlParams::Named(pairs) => {
            let refs: Vec<_> = pairs
                .iter()
                .map(|(column, value)| (column.as_str(), &**value))
                .collect();

            let row_results = stmt.query_map(refs.as_slice(), |row| {
                Ok(convert_sqlite_row_to_nu_value(row, call_span, &columns))
            })?;

            // we collect all rows before returning them. Not ideal but it's hard/impossible to return a stream from a CustomValue
            let mut row_values = vec![];

            for row_result in row_results {
                signals.check(&call_span)?;
                if let Ok(row_value) = row_result {
                    row_values.push(row_value);
                }
            }

            row_values
        }
    };

    Ok(Value::list(row_values, call_span))
}

#[deprecated]
fn read_entire_sqlite_db(
    conn: Connection,
    call_span: Span,
    signals: &Signals,
) -> Result<Value, SqliteOrShellError> {
    let mut tables = Record::new();

    let mut get_table_names =
        conn.prepare("SELECT name FROM sqlite_master WHERE type = 'table'")?;
    let rows = get_table_names.query_map([], |row| row.get(0))?;

    for row in rows {
        let table_name: String = row?;
        // TODO: Should use params here?
        let table_stmt = conn.prepare(&format!("select * from [{table_name}]"))?;
        let rows =
            prepared_statement_to_nu_list(table_stmt, NuSqlParams::default(), call_span, signals)?;
        tables.push(table_name, rows);
    }

    Ok(Value::record(tables, call_span))
}

#[deprecated]
pub fn convert_sqlite_row_to_nu_value(row: &Row, span: Span, columns: &[TypedColumn]) -> Value {
    let record = columns
        .iter()
        .enumerate()
        .map(|(i, col)| {
            (
                col.name.clone(),
                convert_sqlite_value_to_nu_value(row.get_ref_unwrap(i), col.decl_type, span),
            )
        })
        .collect();

    Value::record(record, span)
}

#[deprecated]
pub fn convert_sqlite_value_to_nu_value(
    value: ValueRef,
    decl_type: Option<DeclType>,
    span: Span,
) -> Value {
    match value {
        ValueRef::Null => Value::nothing(span),
        ValueRef::Integer(i) => Value::int(i, span),
        ValueRef::Real(f) => Value::float(f, span),
        ValueRef::Text(buf) => match (std::str::from_utf8(buf), decl_type) {
            (Ok(txt), Some(DeclType::Json | DeclType::Jsonb)) => {
                match crate::convert_json_string_to_value(txt, span) {
                    Ok(val) => val,
                    Err(err) => Value::error(err, span),
                }
            }
            (Ok(txt), _) => Value::string(txt.to_string(), span),
            (Err(_), _) => Value::error(ShellError::NonUtf8 { span }, span),
        },
        ValueRef::Blob(u) => Value::binary(u.to_vec(), span),
    }
}

#[deprecated]
pub fn open_connection_in_memory_custom() -> Result<Connection, DatabaseError> {
    let conn = Connection::open(MEMORY_DB).map_err(|error| DatabaseError::OpenConnection {
        path: DatabasePath::InMemoryCustom,
        error,
    })?;
    conn.busy_handler(Some(SQLiteDatabase::sleeper))
        .map_err(|error| DatabaseError::SetBusyHandler {
            path: DatabasePath::InMemoryCustom,
            error,
        })?;
    Ok(conn)
}

#[deprecated]
pub fn open_connection_in_memory() -> Result<Connection, DatabaseError> {
    let conn = Connection::open_in_memory().map_err(|error| DatabaseError::OpenConnection {
        path: DatabasePath::InMemory,
        error,
    })?;
    Ok(conn)
}

#[cfg(test)]
mod test {
    use super::*;
    use nu_protocol::record;

    #[test]
    fn can_read_empty_db() {
        let db = open_connection_in_memory().unwrap();
        let converted_db = read_entire_sqlite_db(db, Span::test_data(), &Signals::empty()).unwrap();

        let expected = Value::test_record(Record::new());

        assert_eq!(converted_db, expected);
    }

    #[test]
    fn can_read_empty_table() {
        let db = open_connection_in_memory().unwrap();

        db.execute(
            "CREATE TABLE person (
                    id     INTEGER PRIMARY KEY,
                    name   TEXT NOT NULL,
                    data   BLOB
                    )",
            [],
        )
        .unwrap();
        let converted_db = read_entire_sqlite_db(db, Span::test_data(), &Signals::empty()).unwrap();

        let expected = Value::test_record(record! {
            "person" => Value::test_list(vec![]),
        });

        assert_eq!(converted_db, expected);
    }

    #[test]
    fn can_read_null_and_non_null_data() {
        let span = Span::test_data();
        let db = open_connection_in_memory().unwrap();

        db.execute(
            "CREATE TABLE item (
                    id     INTEGER PRIMARY KEY,
                    name   TEXT
                    )",
            [],
        )
        .unwrap();

        db.execute("INSERT INTO item (id, name) VALUES (123, NULL)", [])
            .unwrap();

        db.execute("INSERT INTO item (id, name) VALUES (456, 'foo bar')", [])
            .unwrap();

        let converted_db = read_entire_sqlite_db(db, span, &Signals::empty()).unwrap();

        let expected = Value::test_record(record! {
            "item" => Value::test_list(
                vec![
                    Value::test_record(record! {
                        "id" =>   Value::test_int(123),
                        "name" => Value::nothing(span),
                    }),
                    Value::test_record(record! {
                        "id" =>   Value::test_int(456),
                        "name" => Value::test_string("foo bar"),
                    }),
                ]
            ),
        });

        assert_eq!(converted_db, expected);
    }
}
