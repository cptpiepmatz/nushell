use std::borrow::Cow;

use nu_protocol::{shell_error::location::Location, FromValue, Span, Spanned};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SqlString {
    UserProvided {
        sql: String,
        span: Span,
    },
    Internal {
        sql: Cow<'static, str>,
        location: Location,
    },
}

impl FromValue for SqlString {
    fn from_value(v: nu_protocol::Value) -> Result<Self, nu_protocol::ShellError> {
        let Spanned { item, span } = Spanned::<String>::from_value(v)?;
        Ok(Self::UserProvided { sql: item, span })
    }
}

impl SqlString {
    pub fn new_internal(sql: impl Into<Cow<'static, str>>, location: Location) -> Self {
        Self::Internal {
            sql: sql.into(),
            location,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::UserProvided { sql, .. } => sql,
            Self::Internal { sql, .. } => sql,
        }
    }
}