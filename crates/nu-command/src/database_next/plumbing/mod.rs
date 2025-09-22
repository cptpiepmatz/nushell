use crate::database_next::{error::DatabaseError, plumbing::decl_type::DatabaseDeclType};

use nu_protocol::{Span, Value as NuValue, shell_error::io::IoError};
use rusqlite::types::Value as RusqliteValue;

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

fn nu_value_to_rusqlite_value(
    value: NuValue,
    strict: bool,
) -> Result<RusqliteValue, DatabaseError> {
    let decl_type = DatabaseDeclType::try_from(&value)?;
    if decl_type.as_str(strict).is_none() {
        return Err(DatabaseError::Unsupported {
            r#type: value.get_type(),
            span: value.span(),
        });
    }

    match value {
        // We do *not* handle booleans as integers as its hard to get them out again as booleans
        // this way.
        NuValue::Bool { val, .. } => Ok(RusqliteValue::Text(val.to_string())),
        NuValue::Int { val, .. } => Ok(RusqliteValue::Integer(val)),
        NuValue::Float { val, .. } => Ok(RusqliteValue::Real(val)),
        NuValue::String { val, .. } => Ok(RusqliteValue::Text(val)),
        NuValue::Glob { val, no_expand, .. } => {
            Ok(RusqliteValue::Text(format!("{no_expand}:{val}")))
        }
        NuValue::Filesize { val, .. } => Ok(RusqliteValue::Integer(val.get())),
        NuValue::Duration { val, .. } => Ok(RusqliteValue::Integer(val)),
        NuValue::Date { val, .. } => Ok(RusqliteValue::Text(val.to_rfc3339())),
        NuValue::Binary { val, .. } => Ok(RusqliteValue::Blob(val)),
        NuValue::CellPath { val, .. } => Ok(RusqliteValue::Text(format!("{val}"))),
        NuValue::Nothing { .. } => Ok(RusqliteValue::Null),
        val => match nu_json::to_string(&val) {
            Ok(val) => Ok(RusqliteValue::Text(val)),
            Err(nu_json::Error::Syntax(..)) => unreachable!("we produce valid json syntax"),
            Err(nu_json::Error::FromUtf8(error)) => Err(DatabaseError::FromUtf8 {
                span: val.span(),
                error,
            }),
            Err(nu_json::Error::Io(err)) => {
                Err(DatabaseError::Io(IoError::new_with_additional_context(
                    err,
                    val.span(),
                    None,
                    "Error while converting nu value into database value",
                )))
            }
        },
    }
}

fn rusqlite_value_to_nu_value(
    value: RusqliteValue,
    decl_type: Option<DatabaseDeclType>,
    span: Span,
) -> Result<NuValue, DatabaseError> {
    // alias used types to make match more comprehensive
    use DatabaseDeclType as DDT;
    use NuValue as NV;
    use RusqliteValue as RV;

    match (value, decl_type) {
        (RV::Null, _) => Ok(NV::nothing(span)),
        (RV::Integer(val), Some(DDT::Filesize)) => Ok(NV::filesize(val, span)),
        (RV::Integer(val), Some(DDT::Duration)) => Ok(NV::duration(val, span)),
        (RV::Integer(val), Some(DDT::Int) | None) => Ok(NV::int(val, span)),
        (RV::Real(val), Some(DDT::Float) | None) => Ok(NV::float(val, span)),
        (RV::Blob(val), Some(DDT::Binary) | None) => Ok(NV::binary(val, span)),
        (RV::Text(val), Some(DDT::String)) => Ok(NV::string(val, span)),
        (RV::Text(val), Some(DDT::Bool)) if val == "true" => Ok(NV::bool(true, span)),
        (RV::Text(val), Some(DDT::Bool)) if val == "false" => Ok(NV::bool(false, span)),
        (RV::Text(_val), Some(DDT::Bool)) => Err(DatabaseError::Todo {
            msg: "Handle parsing errors in rusqlite conversion to nu values".into(),
            span,
        }),
        (RV::Text(_val), Some(DDT::Glob)) => Err(DatabaseError::Todo {
            msg: "Implement glob conversion back from rusqlite value".into(),
            span,
        }),
        (RV::Text(_val), Some(DDT::Date)) => Err(DatabaseError::Todo {
            msg: "Implement date conversion back from rusqlite value".into(),
            span,
        }),
        (RV::Text(_val), Some(DDT::CellPath)) => Err(DatabaseError::Todo {
            msg: "Implement cell path parsing to read cell paths from sqlite".into(),
            span,
        }),
        (RusqliteValue::Text(val), _) => match nu_json::from_str::<NuValue>(&val) {
            Ok(val) => Ok(val.with_span(span)),
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
