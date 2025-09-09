use nu_path::AbsolutePathBuf;
use nu_protocol::{CustomValue, FromValue, IntoValue, ShellError, Span, Value};
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, path::{Path, PathBuf}};

use crate::database_next::{connection::DatabaseConnection, error::DatabaseError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseValue {
    storage: DatabaseStorage,
}

impl DatabaseValue {
    pub const TYPE_NAME: &'static str = "database";

    pub fn open_connection(&self) -> Result<DatabaseConnection, DatabaseError> {
        DatabaseConnection::open(self)
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabaseStorage {
    File { path: AbsolutePathBuf, span: Span },
    InMemoryStor,
    InMemoryHistory,
}

impl DatabaseStorage {
    /// Get storage path for the database.
    ///
    /// The return is marked as a [`Path`] as [`Connection::open`](rusqlite::Connection::open) asks
    /// for an [`AsRef<Path>`](AsRef) even though this might contain in memory values like
    /// ":memory:".
    pub fn as_path(&self) -> &Path {
        match self {
            DatabaseStorage::File { path, .. } => path.as_std_path(),
            DatabaseStorage::InMemoryStor => Path::new(":memory:"),
            DatabaseStorage::InMemoryHistory => Path::new("file:memdb1?mode=memory&cache=shared"),
        }
    }
}
