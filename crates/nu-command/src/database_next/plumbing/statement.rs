use nu_protocol::Span;
use rusqlite::{Statement, ToSql};

use crate::database_next::{
    error::DatabaseError,
    plumbing::{params::DatabaseParams, sql::SqlString},
};

#[derive(Debug)]
pub struct DatabaseStatement<'s> {
    pub(super) inner: Statement<'s>,
    pub(super) sql: SqlString,
}

impl<'s> DatabaseStatement<'s> {
    fn sql(&self) -> SqlString {
        let expanded = self.inner.expanded_sql();
        match (&self.sql, expanded) {
            (_, None) => self.sql.clone(),
            (SqlString::UserProvided { span, .. }, Some(sql)) => {
                SqlString::UserProvided { sql, span: *span }
            }
            (SqlString::Internal { location, .. }, Some(sql)) => SqlString::Internal {
                sql: sql.into(),
                location: location.clone(),
            },
        }
    }

    pub fn execute(&mut self, params: DatabaseParams, span: Span) -> Result<usize, DatabaseError> {
        let res = match params {
            DatabaseParams::Unnamed(values) => {
                let params: Vec<_> = values.iter().map(|v| v as &dyn ToSql).collect();
                self.inner.execute(params.as_slice())
            }
            DatabaseParams::Named(values) => {
                let params: Vec<_> = values
                    .iter()
                    .map(|(k, v)| (k.as_str(), v as &dyn ToSql))
                    .collect();
                self.inner.execute(params.as_slice())
            }
        };

        res.map_err(|error| DatabaseError::ExecuteStatement {
            sql: self.sql(),
            span,
            error,
        })
    }
}
