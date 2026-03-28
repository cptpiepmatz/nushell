use std::fmt::Display;

use nu_protocol::{Span, Type as NuType, Value};

use crate::database_nova::error::DatabaseError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseDeclType {
    Any,
    Binary,
    Bool,
    CellPath,
    Date,
    Duration,
    Filesize,
    Float,
    Glob,
    Int,
    List,
    Nothing,
    Record,
    String,
}

impl DatabaseDeclType {
    // strict types
    const STRICT_ANY: &str = "ANY";
    const BINARY: &str = "BLOB";
    const FLOAT: &str = "REAL";
    const INT: &str = "INT";
    const STRING: &str = "TEXT";

    // custom types when not strict
    const NON_STRICT_ANY: &str = "NU ANY BLOB";
    const BOOL: &str = "NU BOOL TEXT";
    const CELLPATH: &str = "NU CELLPATH TEXT";
    const DATE: &str = "NU DATE TEXT";
    const DURATION: &str = "NU DURATION INT";
    const FILESIZE: &str = "NU FILESIZE INT";
    const GLOB: &str = "NU GLOB TEXT";
    const LIST: &str = "NU LIST JSON TEXT";
    const NOTHING: &str = "NU NOTHING BLOB";
    const RECORD: &str = "NU RECORD JSON TEXT";

    #[rustfmt::skip]
    pub fn as_str(&self, strict: bool) -> &str {
        match (self, strict) {
            (Self::Any,      true)  => Self::STRICT_ANY,
            (Self::Any,      false) => Self::NON_STRICT_ANY,
            (Self::Bool,     true)  => Self::STRING,
            (Self::Bool,     false) => Self::BOOL,
            (Self::Int,      _)     => Self::INT,
            (Self::Float,    _)     => Self::FLOAT,
            (Self::String,   _)     => Self::STRING,
            (Self::Glob,     true)  => Self::STRING,
            (Self::Glob,     false) => Self::GLOB,
            (Self::Filesize, true)  => Self::INT,
            (Self::Filesize, false) => Self::FILESIZE,
            (Self::Duration, true)  => Self::INT,
            (Self::Duration, false) => Self::DURATION,
            (Self::Date,     true)  => Self::STRING,
            (Self::Date,     false) => Self::DATE,
            (Self::Record,   true)  => Self::STRING,
            (Self::Record,   false) => Self::RECORD,
            (Self::List,     true)  => Self::STRING,
            (Self::List,     false) => Self::LIST,
            (Self::Binary,   _)     => Self::BINARY,
            (Self::CellPath, true)  => Self::STRING,
            (Self::CellPath, false) => Self::CELLPATH,
            (Self::Nothing,  true)  => Self::STRICT_ANY,
            (Self::Nothing,  false) => Self::NOTHING,
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s.to_uppercase().as_str() {
            Self::STRICT_ANY => Self::Any,
            Self::NON_STRICT_ANY => Self::Any,
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

    pub fn try_from_type(ty: &NuType, span: Span) -> Result<Self, DatabaseError> {
        Ok(match ty {
            NuType::Any => Self::Any,
            NuType::Binary => Self::Binary,
            NuType::Bool => Self::Bool,
            NuType::CellPath => Self::CellPath,
            NuType::Date => Self::Date,
            NuType::Duration => Self::Duration,
            NuType::Filesize => Self::Filesize,
            NuType::Float => Self::Float,
            NuType::Int => Self::Int,
            NuType::List(_) => Self::List,
            NuType::Nothing => Self::Nothing,
            NuType::Number => Self::Float,
            NuType::OneOf(_) => Self::Any,
            NuType::Record(_) => Self::Record,
            NuType::String => Self::String,
            NuType::Glob => Self::Glob,
            NuType::Table(_) => Self::List,
            NuType::Block | NuType::Closure | NuType::Custom(_) | NuType::Error | NuType::Range => {
                return Err(DatabaseError::Unsupported {
                    r#type: ty.clone(),
                    span,
                });
            }
        })
    }
}

impl TryFrom<&Value> for DatabaseDeclType {
    type Error = DatabaseError;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        Ok(match value {
            Value::Bool { .. } => Self::Bool,
            Value::Int { .. } => Self::Int,
            Value::Float { .. } => Self::Float,
            Value::String { .. } => Self::String,
            Value::Glob { .. } => Self::Glob,
            Value::Filesize { .. } => Self::Filesize,
            Value::Duration { .. } => Self::Duration,
            Value::Date { .. } => Self::Date,
            Value::Record { .. } => Self::Record,
            Value::List { .. } => Self::List,
            Value::Binary { .. } => Self::Binary,
            Value::CellPath { .. } => Self::CellPath,
            Value::Nothing { .. } => Self::Nothing,
            // explicitly state these to get an error if we add another value variant
            Value::Range { .. }
            | Value::Closure { .. }
            | Value::Error { .. }
            | Value::Custom { .. } => {
                return Err(DatabaseError::Unsupported {
                    r#type: value.get_type(),
                    span: value.span(),
                });
            }
        })
    }
}

impl Display for DatabaseDeclType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseDeclType::Any => "Any",
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
        }
        .fmt(f)
    }
}
