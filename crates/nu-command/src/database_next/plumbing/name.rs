use nu_protocol::{FromValue, Span, Spanned, location, shell_error::location::Location};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, fmt::Display};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DatabaseName {
    UserProvided {
        name: String,
        span: Span,
    },
    Internal {
        name: Cow<'static, str>,
        location: Location,
    },
}

impl DatabaseName {
    pub const MAIN: DatabaseName = Self::Internal {
        name: Cow::Borrowed("main"),
        location: location!(),
    };

    pub fn new_internal(name: impl Into<Cow<'static, str>>, location: Location) -> Self {
        Self::Internal {
            name: name.into(),
            location,
        }
    }

    #[inline]
    pub fn name(&self) -> &str {
        match self {
            Self::UserProvided { name, .. } => name,
            Self::Internal { name, .. } => name,
        }
    }
}

impl FromValue for DatabaseName {
    fn from_value(v: nu_protocol::Value) -> Result<Self, nu_protocol::ShellError> {
        let Spanned { item, span } = Spanned::<String>::from_value(v)?;
        Ok(Self::UserProvided { name: item, span })
    }
}

impl Display for DatabaseName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.name().fmt(f)
    }
}
