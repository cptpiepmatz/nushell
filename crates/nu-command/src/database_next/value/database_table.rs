use crate::database_next::{
    error::DatabaseError,
    plumbing::{
        connection::DatabaseConnection, name::DatabaseName, storage::DatabaseStorage,
        table::DatabaseTable,
    },
    value::DatabaseValue,
};
use nu_engine::command_prelude::*;
use nu_protocol::{CustomValue, location};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct DatabaseTableValue {
    pub(super) conn: Arc<Mutex<DatabaseConnection>>,
    pub(super) name: DatabaseName,
    pub(super) table: DatabaseTable,
}

impl DatabaseTableValue {
    pub const TYPE_NAME: &'static str = "database-table";

    pub fn is(value: &Value) -> bool {
        let Value::Custom { val, .. } = value else {
            return false;
        };
        val.as_any().is::<Self>()
    }

    pub fn from_database(
        value: DatabaseValue,
        table: DatabaseTable,
        span: Span,
    ) -> Result<Self, DatabaseError> {
        let database_tables = { value.conn.lock().database_tables(&value.name, span)? };
        if database_tables.contains(&table) {
            return Ok(Self {
                conn: value.conn,
                name: value.name,
                table,
            });
        }

        Err(DatabaseError::TableNotFound {
            name: value.name,
            table: table,
            tables: database_tables,
            span: span,
        })
    }
}

#[typetag::serde]
impl CustomValue for DatabaseTableValue {
    fn clone_value(&self, span: Span) -> Value {
        self.clone().into_value(span)
    }

    fn type_name(&self) -> String {
        Self::TYPE_NAME.into()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        let conn = self.conn.lock();
        Ok(conn.read_table(&self.name, &self.table, span)?)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn follow_path_string(
        &self,
        self_span: Span,
        column_name: String,
        path_span: Span,
    ) -> Result<Value, ShellError> {
        let _ = (self_span, column_name, path_span);
        todo!()
    }
}

impl IntoValue for DatabaseTableValue {
    fn into_value(self, span: Span) -> Value {
        Value::custom(Box::new(self), span)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DatabaseTableValueDto {
    storage: DatabaseStorage,
    schema: DatabaseName,
    table: DatabaseTable,
}

impl Serialize for DatabaseTableValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let conn = self.conn.lock();
        let storage = conn.storage().clone();
        let schema = self.name.clone();
        let table = self.table.clone();
        DatabaseTableValueDto {
            storage,
            schema,
            table,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DatabaseTableValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let dto = DatabaseTableValueDto::deserialize(deserializer)?;
        let conn = DatabaseConnection::open_internal(dto.storage, location!())
            .map_err(|err| serde::de::Error::custom(ShellError::from(err).to_string()))?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            name: dto.schema,
            table: dto.table,
        })
    }
}
