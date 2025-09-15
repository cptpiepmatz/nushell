use std::sync::Arc;

use nu_protocol::{CustomValue, FromValue, IntoValue, ShellError, Span, Value, location};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::database_next::plumbing::{connection::DatabaseConnection, storage::DatabaseStorage};

#[derive(Debug, Clone)]
pub struct DatabaseValue {
    conn: Arc<Mutex<DatabaseConnection>>,
}

impl DatabaseValue {
    pub const TYPE_NAME: &'static str = "database";

    pub fn new(conn: DatabaseConnection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    pub fn is(value: &Value) -> bool {
        let Value::Custom { val, .. } = value else { return false };
        val.as_any().is::<DatabaseValue>()
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
        let _ = (self_span, column_name, path_span);
        todo!()
    }
}

impl IntoValue for DatabaseValue {
    fn into_value(self, span: Span) -> Value {
        Value::custom(Box::new(self), span)
    }
}

impl FromValue for DatabaseValue {
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

impl Serialize for DatabaseValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let conn = self.conn.lock();
        let storage = conn.storage().clone();
        DatabaseValueDto { storage }.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DatabaseValue {
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
