use nu_protocol::{Record, Span, Value};
use rusqlite::{Column, Row};

use crate::database_next::{
    error::DatabaseError,
    plumbing::{
        column::DatabaseColumn, decl_type::DatabaseDeclType, rusqlite_value_to_nu_value,
        sql::SqlString,
    },
};

#[derive(Debug)]
pub struct DatabaseRow<'stmt, 'sql> {
    inner: &'stmt Row<'stmt>,
    sql: &'sql SqlString,
}

impl<'stmt, 'sql> DatabaseRow<'stmt, 'sql> {
    pub fn read_all(&self, columns: &[DatabaseColumn], span: Span) -> Result<Value, DatabaseError> {
        let mut record = Record::new();
        for column in columns {
            let index = column.name.as_str();
            let stmt = self.inner.as_ref();
            let value: rusqlite::types::Value =
                self.inner.get(index).map_err(|error| DatabaseError::Get {
                    sql: self.sql.expanded(stmt),
                    index: index.into(),
                    span,
                    error,
                })?;

            let decl_type = column.decl_type;
            let value = rusqlite_value_to_nu_value(value, decl_type, span)?;
            record.push(index, value);
        }

        Ok(Value::record(record, span))
    }
}
