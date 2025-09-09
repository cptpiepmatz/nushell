use nu_protocol::{Span, Spanned};
use rusqlite::Connection;

use crate::database_next::{error::DatabaseError, value::DatabaseValue};

pub struct DatabaseConnection<'v> {
    value: &'v DatabaseValue,
    inner: Connection,
}

// TODO: maybe do more of a InternalDatabaseConnection instead

impl<'v> DatabaseConnection<'v> {
    /// Open a new `DatabaseConnection`.
    /// 
    /// This module is private to the module as you should call [`DatabaseValue::open_connection`] 
    /// to open a connection instead.
    /// But the logic for opening a connection it outside the scope of a basic value.
    pub(super) fn open(value: &DatabaseValue) -> Result<Self, DatabaseError> {
        todo!()
    }

    pub fn prepare() {}

    pub fn execute() {}

    pub fn query() {}

    pub fn call(self, span: Span) -> CalledDatabaseConnection<'v> {
        CalledDatabaseConnection { connection: self, call_span: span }
    }
}

pub struct CalledDatabaseConnection<'v> {
    connection: DatabaseConnection<'v>,
    call_span: Span,
}

impl<'v> CalledDatabaseConnection<'v> {

}

