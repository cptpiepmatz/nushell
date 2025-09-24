use crate::database_next::{error::DatabaseError, plumbing::decl_type::DatabaseDeclType};

use nu_protocol::{Span, Value as NuValue, shell_error::io::IoError};
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

fn nu_value_to_sql_value(
    value: NuValue,
    strict: bool,
) -> Result<SqlValue, DatabaseError> {
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
        NuValue::Bool { val, .. } => Ok(SqlValue::Text(val.to_string())),
        NuValue::Int { val, .. } => Ok(SqlValue::Integer(val)),
        NuValue::Float { val, .. } => Ok(SqlValue::Real(val)),
        NuValue::String { val, .. } => Ok(SqlValue::Text(val)),
        NuValue::Glob { val, no_expand, .. } => {
            Ok(SqlValue::Text(format!("{no_expand}:{val}")))
        }
        NuValue::Filesize { val, .. } => Ok(SqlValue::Integer(val.get())),
        NuValue::Duration { val, .. } => Ok(SqlValue::Integer(val)),
        NuValue::Date { val, .. } => Ok(SqlValue::Text(val.to_rfc3339())),
        NuValue::Binary { val, .. } => Ok(SqlValue::Blob(val.into())),
        NuValue::CellPath { val, .. } => Ok(SqlValue::Text(format!("{val}"))),
        NuValue::Nothing { .. } => Ok(SqlValue::Null),
        val => match nu_json::to_string(&val) {
            Ok(val) => Ok(SqlValue::Text(val)),
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
        (SV::Text(_val), Some(DDT::Bool)) => Err(DatabaseError::Todo {
            msg: "Handle parsing errors in rusqlite conversion to nu values".into(),
            span,
        }),
        (SV::Text(_val), Some(DDT::Glob)) => Err(DatabaseError::Todo {
            msg: "Implement glob conversion back from rusqlite value".into(),
            span,
        }),
        (SV::Text(_val), Some(DDT::Date)) => Err(DatabaseError::Todo {
            msg: "Implement date conversion back from rusqlite value".into(),
            span,
        }),
        (SV::Text(_val), Some(DDT::CellPath)) => Err(DatabaseError::Todo {
            msg: "Implement cell path parsing to read cell paths from sqlite".into(),
            span,
        }),
        (SV::Text(val), _) => match nu_json::from_str::<NuValue>(&val) {
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
