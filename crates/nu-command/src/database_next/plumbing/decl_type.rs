use nu_protocol::Value;

use crate::database_next::error::DatabaseError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseDeclType {
    Bool,
    Int,
    Float,
    String,
    Glob,
    Filesize,
    Duration,
    Date,
    Record,
    List,
    Binary,
    CellPath,
    Nothing,
    Any,
}

impl DatabaseDeclType {
    #[rustfmt::skip]
    pub fn as_str(&self, strict: bool) -> Option<&str> {
        match (self, strict) {
            (Self::Bool,     true)  => Some("TEXT"),
            (Self::Bool,     false) => Some("BOOL TEXT"),
            (Self::Int,      _)     => Some("INT"),
            (Self::Float,    _)     => Some("REAL"),
            (Self::String,   _)     => Some("TEXT"),
            (Self::Glob,     true)  => None,
            (Self::Glob,     false) => Some("GLOB TEXT"),
            (Self::Filesize, true)  => None,
            (Self::Filesize, false) => Some("FILESIZE INT"),
            (Self::Duration, true)  => None,
            (Self::Duration, false) => Some("DURATION INT"),
            (Self::Date,     true)  => None,
            (Self::Date,     false) => Some("DATE TEXT"),
            (Self::Record,   true)  => Some("TEXT"),
            (Self::Record,   false) => Some("RECORD TEXT"),
            (Self::List,     true)  => Some("TEXT"),
            (Self::List,     false) => Some("LIST TEXT"),
            (Self::Binary,   _)     => Some("BLOB"),
            (Self::CellPath, true)  => None,
            (Self::CellPath, false) => Some("CELLPATH TEXT"),
            (Self::Nothing,  true)  => Some("ANY"),
            (Self::Nothing,  false) => Some("NOTHING ANY"),
            (Self::Any,      _)     => Some("ANY"),
        }
    }
}

impl TryFrom<&Value> for DatabaseDeclType {
    type Error = DatabaseError;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Bool { .. } => Ok(Self::Bool),
            Value::Int { .. } => Ok(Self::Int),
            Value::Float { .. } => Ok(Self::Float),
            Value::String { .. } => Ok(Self::String),
            Value::Glob { .. } => Ok(Self::Glob),
            Value::Filesize { .. } => Ok(Self::Filesize),
            Value::Duration { .. } => Ok(Self::Duration),
            Value::Date { .. } => Ok(Self::Date),
            Value::Record { .. } => Ok(Self::Record),
            Value::List { .. } => Ok(Self::List),
            Value::Binary { .. } => Ok(Self::Binary),
            Value::CellPath { .. } => Ok(Self::CellPath),
            Value::Nothing { .. } => Ok(Self::Nothing),
            value => Err(DatabaseError::Unsupported {
                r#type: value.get_type(),
                span: value.span(),
            }),
        }
    }
}
