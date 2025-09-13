use crate::database_next::{error::DatabaseError, plumbing::decl_type::DatabaseDeclType};

use nu_protocol::{Span, Value as NuValue, shell_error::io::IoError};
use rusqlite::{Column, Row, types::Value as RusqliteValue};

pub mod connection;
pub mod decl_type;
pub mod params;
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
        NuValue::Int { val, .. } => Ok(RusqliteValue::Integer(val)),
        NuValue::Float { val, .. } => Ok(RusqliteValue::Real(val)),
        NuValue::String { val, .. } => Ok(RusqliteValue::Text(val)),
        NuValue::Glob { val, no_expand, .. } => Ok(RusqliteValue::Text(format!("{no_expand}:{val}"))),
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

fn rusqlite_value_to_nu_value(value: RusqliteValue, decl_type: DatabaseDeclType, span: Span) -> Result<NuValue, DatabaseError> {
    // TODO: use decl type to infer types better
    match value {
        RusqliteValue::Null => Ok(NuValue::nothing(span)),
        RusqliteValue::Integer(val) => Ok(NuValue::int(val, span)),
        RusqliteValue::Real(val) => Ok(NuValue::float(val, span)),
        RusqliteValue::Blob(val) => Ok(NuValue::binary(val, span)),
        RusqliteValue::Text(val) => match nu_json::from_str::<NuValue>(&val) {
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
    }
}

fn row_to_nu_value<'s>(
    row: &Row<'s>,
    columns: &[Column<'s>],
    span: Span,
) -> Result<NuValue, DatabaseError> {
    for column in columns {
        let value: RusqliteValue = row.get(column.name()).unwrap();
    }

    todo!()
}
