use nu_protocol::Span;
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
        &'s mut self,
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
                unnamed(&mut self.inner, params.as_slice())
            }
            DatabaseParams::Named(values) => {
                let params: Vec<_> = values
                    .iter()
                    .map(|(k, v)| (k.as_str(), v as &dyn ToSql))
                    .collect();
                named(&mut self.inner, params.as_slice())
            }
        }
    }

    pub fn execute(&mut self, params: DatabaseParams, span: Span) -> Result<usize, DatabaseError> {
        self.apply_params(
            params,
            |stmt, params| stmt.execute(params),
            |stmt, params| stmt.execute(params),
        )
        .map_err(|error| DatabaseError::ExecuteStatement {
            sql: self.sql(),
            span,
            error,
        })
    }

    pub fn query_raw(&mut self, params: DatabaseParams, span: Span) -> Result<Rows<'_>, DatabaseError> {
        let self_ptr: *const Self = self;

        self.apply_params(
            params,
            |stmt, p| stmt.query(p),
            |stmt, p| stmt.query(p),
        )
        .map_err(|error| {
            // SAFETY:
            // - Runs only on Err, no `Rows<'_>`, so no active `&mut self` borrow.
            // - We don't move `self` in this fn, so `self_ptr` stays valid.
            // - `sql()` needs only `&self` and does not mutate.
            // - `map_err` can't take `&self` (Ok holds `Rows<'_>` borrowing `self`), so we read via `self_ptr`.
            //
            // ALTERNATIVE:
            // - Call `self.sql()` before `apply_params`, but that may clone strings/span on the fast path.
            let sql = unsafe { (&*self_ptr).sql() };
            DatabaseError::QueryStatement { sql, span, error }
        })
    }

    pub fn query(&mut self, params: DatabaseParams, span: Span) -> Result<(), DatabaseError> {
        let _ = (params, span);
        todo!()
    }
}
