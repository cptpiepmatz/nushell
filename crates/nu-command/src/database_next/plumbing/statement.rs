use nu_protocol::{Span, Value};
use rusqlite::{Rows, Statement, ToSql};

use crate::database_next::{
    error::DatabaseError,
    plumbing::{column::DatabaseColumn, params::DatabaseParams, row::DatabaseRow, sql::SqlString},
};

#[derive(Debug)]
pub struct DatabaseStatement<'c> {
    inner: Statement<'c>,
    sql: SqlString,
}

impl<'c> DatabaseStatement<'c> {
    pub(super) fn new(stmt: Statement<'c>, sql: SqlString) -> Self {
        Self { inner: stmt, sql }
    }

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

    pub fn readonly(&self) -> bool {
        self.inner.readonly()
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
        let columns = self
            .inner
            .columns()
            .into_iter()
            .map(DatabaseColumn::from)
            .collect::<Vec<_>>();
        let mut rows = Self::query_rows(&mut self.inner, &self.sql, params, span)?;

        let mut values = Vec::new();
        for index in 0.. {
            match rows.next() {
                Ok(None) => break,
                Ok(Some(row)) => {
                    let row = DatabaseRow::new(row, &self.sql);
                    let record = row.read_all(&columns, span)?;
                    values.push(record);
                }
                Err(error) => {
                    let sql = match rows.as_ref() {
                        Some(stmt) => self.sql.expanded(stmt),
                        None => self.sql.clone(),
                    };
                    return Err(DatabaseError::Iterate {
                        sql,
                        index,
                        span,
                        error,
                    });
                }
            }
        }

        Ok(Value::list(values, span))
    }
}
