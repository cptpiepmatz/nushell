use rusqlite::Statement;

use crate::database_next::{error::DatabaseError, plumbing::sql::SqlString};

#[derive(Debug)]
pub struct DatabaseStatement<'s> {
    pub(super) inner: Statement<'s>,
    pub(super) sql: SqlString,
}

impl<'s> DatabaseStatement<'s> {
    pub fn execute(&self) -> Result<(), DatabaseError> {
        // self.inner.execute(params)
        
        todo!()
    }
}