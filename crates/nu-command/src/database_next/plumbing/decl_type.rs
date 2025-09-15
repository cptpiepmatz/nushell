use std::fmt::Display;

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
}

impl DatabaseDeclType {
    // strict types
    const INT: &str = "INT";
    const FLOAT: &str = "REAL";
    const STRING: &str = "TEXT";
    const BINARY: &str = "BLOB";
    const ANY: &str = "ANY";

    // custom types when not strict
    const BOOL: &str = "NU BOOL TEXT";
    const GLOB: &str = "NU GLOB TEXT";
    const FILESIZE: &str = "NU FILESIZE INT";
    const DURATION: &str = "NU DURATION INT";
    const DATE: &str = "NU DATE TEXT";
    const RECORD: &str = "NU RECORD JSON TEXT";
    const LIST: &str = "NU LIST JSON TEXT";
    const CELLPATH: &str = "NU CELLPATH TEXT";
    const NOTHING: &str = "NU NOTHING ANY";

    #[rustfmt::skip]
    pub fn as_str(&self, strict: bool) -> Option<&str> {
        match (self, strict) {
            (Self::Bool,     true)  => Some(Self::STRING),
            (Self::Bool,     false) => Some(Self::BOOL),
            (Self::Int,      _)     => Some(Self::INT),
            (Self::Float,    _)     => Some(Self::FLOAT),
            (Self::String,   _)     => Some(Self::STRING),
            (Self::Glob,     true)  => None,
            (Self::Glob,     false) => Some(Self::GLOB),
            (Self::Filesize, true)  => None,
            (Self::Filesize, false) => Some(Self::FILESIZE),
            (Self::Duration, true)  => None,
            (Self::Duration, false) => Some(Self::DURATION),
            (Self::Date,     true)  => None,
            (Self::Date,     false) => Some(Self::DATE),
            (Self::Record,   true)  => Some(Self::STRING),
            (Self::Record,   false) => Some(Self::RECORD),
            (Self::List,     true)  => Some(Self::STRING),
            (Self::List,     false) => Some(Self::LIST),
            (Self::Binary,   _)     => Some(Self::BINARY),
            (Self::CellPath, true)  => None,
            (Self::CellPath, false) => Some(Self::CELLPATH),
            (Self::Nothing,  true)  => Some(Self::ANY),
            (Self::Nothing,  false) => Some(Self::NOTHING),
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s.to_uppercase().as_str() {
            Self::BOOL => Self::Bool,
            Self::INT => Self::Int,
            Self::FLOAT => Self::Float,
            Self::STRING => Self::String,
            Self::GLOB => Self::Glob,
            Self::FILESIZE => Self::Filesize,
            Self::DURATION => Self::Duration,
            Self::DATE => Self::Date,
            Self::RECORD => Self::Record,
            Self::LIST => Self::List,
            Self::BINARY => Self::Binary,
            Self::CELLPATH => Self::CellPath,
            Self::NOTHING => Self::Nothing,
            _ => return None,
        })
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
            // explicitly state these to get an error if we add another value variant
            Value::Range { .. }
            | Value::Closure { .. }
            | Value::Error { .. }
            | Value::Custom { .. } => Err(DatabaseError::Unsupported {
                r#type: value.get_type(),
                span: value.span(),
            }),
        }
    }
}

impl Display for DatabaseDeclType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseDeclType::Bool => "Bool",
            DatabaseDeclType::Int => "Int",
            DatabaseDeclType::Float => "Float",
            DatabaseDeclType::String => "String",
            DatabaseDeclType::Glob => "Glob",
            DatabaseDeclType::Filesize => "Filesize",
            DatabaseDeclType::Duration => "Duration",
            DatabaseDeclType::Date => "Date",
            DatabaseDeclType::Record => "Record",
            DatabaseDeclType::List => "List",
            DatabaseDeclType::Binary => "Binary",
            DatabaseDeclType::CellPath => "CellPath",
            DatabaseDeclType::Nothing => "Nothing",
        }.fmt(f)
    }
}
