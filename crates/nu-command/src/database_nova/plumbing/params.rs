use crate::database_nova::{
    error::DatabaseError,
    plumbing::{nu_value_to_sql_value, value::SqlValue},
};

pub enum DatabaseParams {
    Unnamed(Vec<SqlValue>),
    Named(Vec<(String, SqlValue)>),
}

impl DatabaseParams {
    pub fn new_empty() -> Self {
        Self::Unnamed(vec![])
    }

    pub fn new_unnamed(
        iter: impl Iterator<Item = nu_protocol::Value>,
    ) -> Result<Self, DatabaseError> {
        let (min, max) = iter.size_hint();
        let capacity = max.unwrap_or(min);
        let mut values = Vec::with_capacity(capacity);
        for value in iter {
            let value = nu_value_to_sql_value(value)?;
            values.push(value);
        }
        Ok(Self::Unnamed(values))
    }

    pub fn new_named(
        iter: impl Iterator<Item = (String, nu_protocol::Value)>,
    ) -> Result<Self, DatabaseError> {
        let (min, max) = iter.size_hint();
        let capacity = max.unwrap_or(min);
        let mut values = Vec::with_capacity(capacity);
        for (key, value) in iter {
            let value = nu_value_to_sql_value(value)?;
            values.push((key, value));
        }
        Ok(Self::Named(values))
    }
}
