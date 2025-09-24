use bytes::{Buf, Bytes};
use rusqlite::{types::{FromSql, FromSqlResult, ToSqlOutput, Type, ValueRef}, ToSql};

/// Replacement type for [`rusqlite::types::Value`].
///
/// The conversion from [`rusqlite::types::ValueRef`] to [`rusqlite::types::Value`] might panic on
/// non utf-8 strings.
/// To avoid crashes this type is used instead.
/// To ensure the [`Value`](rusqlite::types::Value) is never used, a clippy lint is applied
/// disallowing the usage of that type.
/// Similar to [`Value`](rusqlite::types::Value), this acts as an owned variant of
/// [`ValueRef`](rusqlite::types::ValueRef).
/// 
/// The naming of this value doesn't follow the typical `Database*` schema of the 
/// [`plumbing`](crate::database_next::plumbing) module as it would clash with 
/// [`database_next::value::DatabaseValue`](crate::database_next::value::DatabaseValue).
#[derive(Debug, Clone, PartialEq)]
pub enum SqlValue {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Bytes),
}

impl SqlValue {
    pub fn data_type(&self) -> Type {
        match self {
            Self::Null => Type::Null,
            Self::Integer(..) => Type::Integer,
            Self::Real(..) => Type::Real,
            Self::Text(..) => Type::Text,
            Self::Blob(..) => Type::Blob,
        }
    }

    pub fn as_ref(&self) -> ValueRef {
        match self {
            SqlValue::Null => ValueRef::Null,
            SqlValue::Integer(int) => ValueRef::Integer(*int),
            SqlValue::Real(real) => ValueRef::Real(*real),
            SqlValue::Text(string) => ValueRef::Text(string.as_bytes()),
            SqlValue::Blob(bytes) => ValueRef::Blob(&bytes),
        }
    }
}

impl FromSql for SqlValue {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        Ok(match value {
            ValueRef::Null => Self::Null,
            ValueRef::Integer(int) => Self::Integer(int),
            ValueRef::Real(real) => Self::Real(real),
            ValueRef::Blob(bytes) => Self::Blob(bytes.to_owned().into()),
            ValueRef::Text(bytes) => {
                // probably utf8, most is
                if let Ok(string) = str::from_utf8(bytes) {
                    return Ok(Self::Text(string.to_owned()));
                }

                // maybe utf16, likely on Windows
                if let Ok(utf16_bytes) = bytemuck::try_cast_slice(bytes)
                    && let Ok(string) = String::from_utf16(utf16_bytes)
                {
                    return Ok(Self::Text(string));
                }

                // just give me some text (っ °Д °;)っ
                Self::Text(String::from_utf8_lossy(bytes).into())
            }
        })
    }
}

impl ToSql for SqlValue {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Borrowed(self.as_ref()))
    }
}
