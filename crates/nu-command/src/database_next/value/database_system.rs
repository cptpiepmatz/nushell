use std::{borrow::Cow, sync::Arc};

use nu_engine::command_prelude::*;
use nu_protocol::{CustomValue, location};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::database_next::{
    plumbing::{connection::DatabaseConnection, name::DatabaseName, storage::DatabaseStorage},
    value::DatabaseValue,
};

#[derive(Debug, Clone)]
pub struct DatabaseSystemValue {
    conn: Arc<Mutex<DatabaseConnection>>,
}

impl DatabaseSystemValue {
    pub const TYPE_NAME: &'static str = "database-system";

    pub fn new(conn: DatabaseConnection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    pub fn is(value: &Value) -> bool {
        let Value::Custom { val, .. } = value else {
            return false;
        };
        val.as_any().is::<DatabaseSystemValue>()
    }

    pub fn database(&self, name: DatabaseName, span: Span) -> Result<DatabaseValue, ShellError> {
        DatabaseValue::new(self.conn.clone(), name, span)
    }
}

#[typetag::serde]
impl CustomValue for DatabaseSystemValue {
    fn clone_value(&self, span: Span) -> Value {
        self.clone().into_value(span)
    }

    fn type_name(&self) -> String {
        Self::TYPE_NAME.into()
    }

    fn to_base_value(&self, span: Span) -> Result<Value, ShellError> {
        let conn = self.conn.lock();
        Ok(conn.read_all(span)?)
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
        let name = DatabaseName::UserProvided {
            name: column_name,
            span: path_span,
        };
        Ok(Value::custom(
            Box::new(self.database(name, self_span)?),
            self_span,
        ))
    }
}

impl IntoValue for DatabaseSystemValue {
    fn into_value(self, span: Span) -> Value {
        Value::custom(Box::new(self), span)
    }
}

impl FromValue for DatabaseSystemValue {
    fn from_value(v: Value) -> Result<Self, ShellError> {
        if let Value::Custom { val, .. } = &v
            && let Some(val) = val.as_any().downcast_ref::<Self>()
        {
            return Ok(val.clone());
        }

        let span = v.span();
        let conn = DatabaseConnection::open_from_value(v, span)?;
        Ok(Self::new(conn))
    }

    fn expected_type() -> nu_protocol::Type {
        nu_protocol::Type::Custom(Self::TYPE_NAME.to_string().into_boxed_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DatabaseValueDto {
    storage: DatabaseStorage,
}

impl Serialize for DatabaseSystemValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let conn = self.conn.lock();
        let storage = conn.storage().clone();
        DatabaseValueDto { storage }.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DatabaseSystemValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let dto = DatabaseValueDto::deserialize(deserializer)?;
        let conn = DatabaseConnection::open_internal(dto.storage, location!())
            .map_err(|err| serde::de::Error::custom(ShellError::from(err).to_string()))?;
        Ok(Self::new(conn))
    }
}
