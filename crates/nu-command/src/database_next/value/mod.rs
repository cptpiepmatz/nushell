use nu_path::AbsolutePathBuf;
use nu_protocol::{CustomValue, FromValue, IntoValue, ShellError, Span, Value};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, path::{Path, PathBuf}};

use crate::database_next::{connection::DatabaseConnection, error::DatabaseError, storage::DatabaseStorage};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseValue {
    storage: DatabaseStorage,
}

impl DatabaseValue {
    pub const TYPE_NAME: &'static str = "database";
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
        todo!()
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
        if let Value::Custom { val, .. } = v {
            return match val.as_any().downcast_ref::<Self>() {
                Some(database_value) => Ok(database_value.clone()),
                None => todo!()
            }
        }

        todo!()
    }

    fn expected_type() -> nu_protocol::Type {
        nu_protocol::Type::Custom(
            Self::TYPE_NAME
                .to_string()
                .into_boxed_str(),
        )
    }
}

impl AsRef<DatabaseStorage> for DatabaseValue {
    fn as_ref(&self) -> &DatabaseStorage {
        &self.storage
    }
}
