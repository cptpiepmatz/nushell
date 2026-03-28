use nu_protocol::{FromValue, Span, Spanned};
use nu_utils::location::Location;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, fmt::Display};

use crate::database_nova::plumbing::SqlIdentifier;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabaseTableName {
    UserProvided {
        name: String,
        span: Span,
    },
    Internal {
        name: Cow<'static, str>,
        location: Location,
    },
}

impl DatabaseTableName {
    #[track_caller]
    pub fn new_internal(name: impl Into<Cow<'static, str>>) -> Self {
        Self::Internal {
            name: name.into(),
            location: Location::caller(),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::UserProvided { name, .. } => name,
            Self::Internal { name, .. } => name,
        }
    }

    pub fn sql_name(&self) -> impl Display + '_ {
        SqlIdentifier(self.as_str())
    }
}

impl FromValue for DatabaseTableName {
    fn from_value(v: nu_protocol::Value) -> Result<Self, nu_protocol::ShellError> {
        let Spanned { item, span } = Spanned::<String>::from_value(v)?;
        Ok(Self::UserProvided { name: item, span })
    }
}

impl Display for DatabaseTableName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

impl PartialEq for DatabaseTableName {
    fn eq(&self, other: &Self) -> bool {
        let left = self.as_str();
        let right = other.as_str();
        left == right
    }
}

impl Eq for DatabaseTableName {}
