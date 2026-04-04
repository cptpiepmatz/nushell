use std::fmt::{Display, Write};

use crate::database_nova::{error::DatabaseError, plumbing::decl_type::DatabaseDeclType};

use chrono::DateTime;
use nu_protocol::{FromValue, IntoValue, Span, Value as NuValue, shell_error::io::IoError};
use value::SqlValue;

pub mod column;
pub mod connection;
pub mod decl_type;
pub mod list;
pub mod name;
pub mod params;
pub mod row;
pub mod sql;
pub mod statement;
pub mod storage;
pub mod table;
pub mod uri;
pub mod value;

fn nu_value_to_sql_value(value: NuValue) -> Result<SqlValue, DatabaseError> {
    match value {
        // We do *not* handle booleans as integers as it's hard to get them out again as booleans
        // this way.
        NuValue::Bool { val, .. } => Ok(SqlValue::Text(val.to_string())),
        NuValue::Int { val, .. } => Ok(SqlValue::Integer(val)),
        NuValue::Float { val, .. } => Ok(SqlValue::Real(val)),
        NuValue::String { val, .. } => Ok(SqlValue::Text(val)),
        NuValue::Glob { val, no_expand, .. } => Ok(SqlValue::Text(format!("{no_expand}:{val}"))),
        NuValue::Filesize { val, .. } => Ok(SqlValue::Integer(val.get())),
        NuValue::Duration { val, .. } => Ok(SqlValue::Integer(val)),
        NuValue::Date { val, .. } => Ok(SqlValue::Text(val.to_rfc3339())),
        NuValue::Binary { val, .. } => Ok(SqlValue::Blob(val.into())),
        NuValue::CellPath { val, .. } => Ok(SqlValue::Text(format!("{val}"))),
        NuValue::Nothing { .. } => Ok(SqlValue::Null),
        val => {
            let span = val.span();
            let val = nu_json::Value::from_value(val).map_err(DatabaseError::Shell)?;
            match nu_json::to_string(&val) {
                Ok(val) => Ok(SqlValue::Text(val)),
                Err(nu_json::Error::Syntax(..)) => unreachable!("we produce valid json syntax"),
                Err(nu_json::Error::FromUtf8(error)) => {
                    Err(DatabaseError::FromUtf8 { span, error })
                }
                Err(nu_json::Error::Io(err)) => {
                    Err(DatabaseError::Io(IoError::new_with_additional_context(
                        err,
                        span,
                        None,
                        "Error while converting nu value into database value",
                    )))
                }
            }
        }
    }
}

fn sql_value_to_nu_value(
    value: SqlValue,
    decl_type: Option<DatabaseDeclType>,
    span: Span,
) -> Result<NuValue, DatabaseError> {
    // alias used types to make match more comprehensive
    use DatabaseDeclType as DDT;
    use NuValue as NV;
    use SqlValue as SV;

    match (value, decl_type) {
        (SV::Null, _) => Ok(NV::nothing(span)),
        (SV::Integer(val), Some(DDT::Filesize)) => Ok(NV::filesize(val, span)),
        (SV::Integer(val), Some(DDT::Duration)) => Ok(NV::duration(val, span)),
        (SV::Integer(val), Some(DDT::Int) | None) => Ok(NV::int(val, span)),
        (SV::Real(val), Some(DDT::Float) | None) => Ok(NV::float(val, span)),
        (SV::Blob(val), Some(DDT::Binary) | None) => Ok(NV::binary(val, span)),
        (SV::Text(val), Some(DDT::String)) => Ok(NV::string(val, span)),
        (SV::Text(val), Some(DDT::Bool)) if val == "true" => Ok(NV::bool(true, span)),
        (SV::Text(val), Some(DDT::Bool)) if val == "false" => Ok(NV::bool(false, span)),
        (SV::Text(val), Some(DDT::Bool)) => Err(DatabaseError::ParseValue {
            raw: val,
            expected: DDT::Bool,
            msg: "Expected `true` or `false`".into(),
            span,
        }),
        (SV::Text(val), Some(DDT::Glob)) => match val.split_once(':') {
            Some(("true", val)) => Ok(NV::glob(val, true, span)),
            Some(("false", val)) => Ok(NV::glob(val, false, span)),
            Some((b, _)) => Err(DatabaseError::ParseValue {
                raw: val,
                expected: DDT::Glob,
                msg: "Expected `true` or `false` before separator".into(),
                span,
            }),
            None => Err(DatabaseError::ParseValue {
                raw: val,
                expected: DDT::Glob,
                msg: "Invalid format, expected schema `{no_expand}:{glob}`".into(),
                span,
            }),
        },
        (SV::Text(val), Some(DDT::Date)) => match DateTime::parse_from_rfc3339(&val) {
            Ok(dt) => Ok(NV::date(dt, span)),
            Err(err) => Err(DatabaseError::ParseValue {
                raw: val,
                expected: DDT::Date,
                msg: err.to_string().into(),
                span,
            }),
        },
        (SV::Text(val), Some(DDT::CellPath)) => match nuon::from_nuon(&val, Some(span)) {
            Err(err) => Err(DatabaseError::Shell(err)),
            Ok(val @ NV::CellPath { .. }) => Ok(val),
            Ok(_) => Err(DatabaseError::ParseValue {
                raw: val,
                expected: DDT::CellPath,
                msg: "Could be parsed as a nushell value, but not as a cell path".into(),
                span,
            }),
        },
        (SV::Text(val), _) => match nu_json::from_str::<nu_json::Value>(&val) {
            Ok(val) => Ok(val.into_value(span)),
            Err(nu_json::Error::Syntax(..)) => Ok(NuValue::string(val, span)),
            Err(nu_json::Error::FromUtf8(error)) => Err(DatabaseError::FromUtf8 { span, error }),
            Err(nu_json::Error::Io(err)) => {
                Err(DatabaseError::Io(IoError::new_with_additional_context(
                    err,
                    span,
                    None,
                    "Error while converting database value into nu value",
                )))
            }
        },
        (value, Some(decl_type)) => Err(DatabaseError::InvalidDeclType {
            rusqlite_type: value.data_type(),
            decl_type,
            span,
        }),
    }
}

struct SqlIdentifier<'s>(&'s str);

impl<'s> Display for SqlIdentifier<'s> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char('"')?;

        for c in self.0.chars() {
            match c {
                '"' => f.write_str("\"\"")?,
                _ => f.write_char(c)?,
            }
        }

        f.write_char('"')
    }
}
