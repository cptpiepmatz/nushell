use nu_protocol::{FromValue, IntoValue, ShellError, Span};
use rusqlite::{
    ToSql,
    types::{ToSqlOutput, ValueRef},
};

use crate::database::values::sqlite::DeclType;

pub struct ValueDto(pub nu_protocol::Value);

impl IntoValue for ValueDto {
    fn into_value(self, span: nu_protocol::Span) -> nu_protocol::Value {
        self.0.into_value(span)
    }
}

impl FromValue for ValueDto {
    fn from_value(v: nu_protocol::Value) -> Result<Self, nu_protocol::ShellError> {
        Ok(ValueDto(v))
    }

    fn expected_type() -> nu_protocol::Type {
        nu_protocol::Value::expected_type()
    }
}

impl ToSql for ValueDto {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        match &self.0 {
            nu_protocol::Value::Bool { val, .. } => val.to_sql(),
            nu_protocol::Value::Int { val, .. } => val.to_sql(),
            nu_protocol::Value::Float { val, .. } => val.to_sql(),
            nu_protocol::Value::Filesize { val, .. } => Ok(ToSqlOutput::Owned(
                rusqlite::types::Value::Integer(val.get()),
            )),
            nu_protocol::Value::Duration { val, .. } => val.to_sql(),
            nu_protocol::Value::Date { val, .. } => val.to_sql(),
            nu_protocol::Value::String { val, .. } => val.to_sql(),
            nu_protocol::Value::Binary { val, .. } => val.to_sql(),
            nu_protocol::Value::Nothing { .. } => Ok(ToSqlOutput::Borrowed(ValueRef::Null)),
            val => nu_json::to_string(&val)
                .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))
                .map(|s| ToSqlOutput::Owned(rusqlite::types::Value::Text(s))),
        }
    }
}

impl ValueDto {
    pub fn from_value_ref(
        value_ref: ValueRef<'_>,
        decl_type: Option<DeclType>,
        span: Span,
    ) -> ValueDto {
        use nu_protocol::Value;

        let inner = match value_ref {
            ValueRef::Null => Value::nothing(span),
            ValueRef::Integer(i) => Value::int(i, span),
            ValueRef::Real(f) => Value::float(f, span),
            ValueRef::Text(buf) => match (std::str::from_utf8(buf), decl_type) {
                (Ok(txt), Some(DeclType::Json | DeclType::Jsonb)) => {
                    match crate::convert_json_string_to_value(txt, span) {
                        Ok(val) => val,
                        Err(err) => Value::error(err, span),
                    }
                }
                (Ok(txt), _) => Value::string(txt.to_string(), span),
                (Err(_), _) => Value::error(ShellError::NonUtf8 { span }, span),
            },
            ValueRef::Blob(u) => Value::binary(u.to_vec(), span),
        };

        ValueDto(inner)
    }
}
