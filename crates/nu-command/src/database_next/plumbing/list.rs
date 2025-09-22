use nu_protocol::{FromValue, IntoValue, Span, Type, Value};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, IntoValue, FromValue, Serialize, Deserialize)]
pub struct DatabaseListEntry {
    pub seq: i32,
    pub name: String,
    pub file: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatabaseList(Vec<DatabaseListEntry>);

impl DatabaseList {
    pub fn has_database(&self, name: &str) -> bool {
        self.0.iter().any(|entry| entry.name == name)
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &DatabaseListEntry> {
        self.0.iter()
    }
}

impl IntoIterator for DatabaseList {
    type Item = DatabaseListEntry;

    type IntoIter = <Vec<Self::Item> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl IntoValue for DatabaseList {
    fn into_value(self, span: Span) -> Value {
        self.0.into_value(span)
    }
}

impl FromValue for DatabaseList {
    fn from_value(v: Value) -> Result<Self, nu_protocol::ShellError> {
        Ok(Self(FromValue::from_value(v)?))
    }

    fn expected_type() -> Type {
        Type::list(DatabaseListEntry::expected_type())
    }
}
