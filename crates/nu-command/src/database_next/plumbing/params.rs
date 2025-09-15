use crate::database_next::{error::DatabaseError, plumbing::nu_value_to_rusqlite_value};

pub enum DatabaseParams {
    Unnamed(Vec<rusqlite::types::Value>),
    Named(Vec<(String, rusqlite::types::Value)>),
}

impl DatabaseParams {
    pub fn new_empty() -> Self {
        Self::Unnamed(vec![])
    }

    pub fn new_unnamed(
        iter: impl ExactSizeIterator<Item = nu_protocol::Value>,
    ) -> Result<Self, DatabaseError> {
        let mut values = Vec::with_capacity(iter.len());
        for value in iter {
            let value = nu_value_to_rusqlite_value(value, false)?;
            values.push(value);
        }
        Ok(Self::Unnamed(values))
    }

    pub fn new_named(
        iter: impl ExactSizeIterator<Item = (String, nu_protocol::Value)>,
    ) -> Result<Self, DatabaseError> {
        let mut values = Vec::with_capacity(iter.len());
        for (key, value) in iter {
            let value = nu_value_to_rusqlite_value(value, false)?;
            values.push((key, value));
        }
        Ok(Self::Named(values))
    }
}
