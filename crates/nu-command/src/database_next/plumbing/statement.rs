use nu_protocol::{Span, Value};
use rusqlite::{Rows, Statement, ToSql};

use crate::database_next::{
    error::DatabaseError,
    plumbing::{params::DatabaseParams, sql::SqlString},
};

#[derive(Debug)]
pub struct DatabaseStatement<'c> {
    pub(super) inner: Statement<'c>,
    pub(super) sql: SqlString,
}

impl<'c> DatabaseStatement<'c> {
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

    #[inline]
    fn apply_params<'s, T, FU, FN>(
        stmt: &'s mut Statement<'c>,
        params: DatabaseParams,
        mut unnamed: FU,
        mut named: FN,
    ) -> Result<T, rusqlite::Error>
    where
        T: 's,
        FU: FnMut(&'s mut Statement<'_>, &[&dyn ToSql]) -> Result<T, rusqlite::Error>,
        FN: FnMut(&'s mut Statement<'_>, &[(&str, &dyn ToSql)]) -> Result<T, rusqlite::Error>,
    {
        match params {
            DatabaseParams::Unnamed(values) => {
                let params: Vec<_> = values.iter().map(|v| v as &dyn ToSql).collect();
                unnamed(stmt, params.as_slice())
            }
            DatabaseParams::Named(values) => {
                let params: Vec<_> = values
                    .iter()
                    .map(|(k, v)| (k.as_str(), v as &dyn ToSql))
                    .collect();
                named(stmt, params.as_slice())
            }
        }
    }

    pub fn execute(&mut self, params: DatabaseParams, span: Span) -> Result<usize, DatabaseError> {
        Self::apply_params(
            &mut self.inner,
            params,
            |stmt, params| stmt.execute(params),
            |stmt, params| stmt.execute(params),
        )
        .map_err(|error| DatabaseError::ExecuteStatement {
            sql: self.sql.clone(),
            span,
            error,
        })
    }

    #[inline]
    fn query_rows<'s>(
        stmt: &'s mut Statement<'c>,
        sql: &SqlString,
        params: DatabaseParams,
        span: Span,
    ) -> Result<Rows<'s>, DatabaseError> {
        Self::apply_params(
            stmt,
            params,
            |stmt, p| stmt.query(p),
            |stmt, p| stmt.query(p),
        )
        .map_err(|error| DatabaseError::QueryStatement {
            sql: sql.clone(),
            span,
            error,
        })
    }

    pub fn query(&mut self, params: DatabaseParams, span: Span) -> Result<Value, DatabaseError> {
        let mut rows = Self::query_rows(&mut self.inner, &self.sql, params, span)?;
        let row = rows.next().unwrap().unwrap();
        let sql = self.sql.expanded(row.as_ref());
        todo!()
    }
}
