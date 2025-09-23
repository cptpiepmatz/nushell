use crate::database_next::{
    error::DatabaseError,
    plumbing::{
        connection::DatabaseConnection, name::DatabaseName, storage::DatabaseStorage,
        table::DatabaseTable,
    },
    value::DatabaseTableValue,
};
use nu_engine::command_prelude::*;
use nu_protocol::{CustomValue, casing::Casing, location};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct DatabaseValue {
    pub(super) conn: Arc<Mutex<DatabaseConnection>>,
    pub(super) name: DatabaseName,
}

impl DatabaseValue {
    pub const TYPE_NAME: &'static str = "database";

    pub fn new(
        conn: Arc<Mutex<DatabaseConnection>>,
        name: DatabaseName,
        span: Span,
    ) -> Result<Self, ShellError> {
        let db_name = name.name();
        if db_name == "main" {
            return Ok(Self { conn, name });
        }

        let database_list = { conn.lock().database_list(span)? };
        if database_list.has_database(db_name) {
            return Ok(Self { conn, name });
        }

        Err(ShellError::from(DatabaseError::DatabaseNotFound {
            name,
            database_list,
            span,
        }))
    }

    pub fn is(value: &Value) -> bool {
        let Value::Custom { val, .. } = value else {
            return false;
        };
        val.as_any().is::<Self>()
    }

    pub fn with_table(
        self,
        table: DatabaseTable,
        span: Span,
    ) -> Result<DatabaseTableValue, DatabaseError> {
        DatabaseTableValue::from_database(self, table, span)
    }
}

#[typetag::serde]
impl CustomValue for DatabaseValue {
    fn clone_value(&self, span: Span) -> Value {
        self.clone().into_value(span)
    }

    fn type_name(&self) -> String {
        Self::TYPE_NAME.into()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        let conn = self.conn.lock();
        Ok(conn.read_database(&self.name, span)?)
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
        _optional: bool,
        _casing: Casing,
    ) -> Result<Value, ShellError> {
        let table = DatabaseTable::UserProvided {
            name: column_name,
            span: path_span,
        };
        let value = self.clone().with_table(table, self_span)?;
        Ok(Value::custom(Box::new(value), self_span))
    }
}

impl IntoValue for DatabaseValue {
    fn into_value(self, span: Span) -> Value {
        Value::custom(Box::new(self), span)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DatabaseTableValueDto {
    storage: DatabaseStorage,
    schema: DatabaseName,
}

impl Serialize for DatabaseValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let conn = self.conn.lock();
        let storage = conn.storage().clone();
        let schema = self.name.clone();
        DatabaseTableValueDto { storage, schema }.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DatabaseValue {
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
        })
    }
}
