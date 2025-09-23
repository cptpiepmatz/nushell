use nu_protocol::{FromValue, Span, Spanned, shell_error::location::Location};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, fmt::Display};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabaseTable {
    UserProvided {
        name: String,
        span: Span,
    },
    Internal {
        name: Cow<'static, str>,
        location: Location,
    },
}

impl DatabaseTable {
    pub fn new_internal(name: impl Into<Cow<'static, str>>, location: Location) -> Self {
        Self::Internal {
            name: name.into(),
            location,
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::UserProvided { name, .. } => name,
            Self::Internal { name, .. } => name,
        }
    }
}

impl FromValue for DatabaseTable {
    fn from_value(v: nu_protocol::Value) -> Result<Self, nu_protocol::ShellError> {
        let Spanned { item, span } = Spanned::<String>::from_value(v)?;
        Ok(Self::UserProvided { name: item, span })
    }
}

impl Display for DatabaseTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

impl PartialEq for DatabaseTable {
    fn eq(&self, other: &Self) -> bool {
        let left = self.as_str();
        let right = other.as_str();
        left == right
    }
}

impl Eq for DatabaseTable {}
