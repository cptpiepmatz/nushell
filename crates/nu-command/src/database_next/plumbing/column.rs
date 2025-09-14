use rusqlite::Column;

use crate::database_next::plumbing::decl_type::DatabaseDeclType;

#[derive(Debug)]
pub struct DatabaseColumn {
    pub(super) name: String,
    pub(super) decl_type: Option<DatabaseDeclType>,
}

impl<'s> From<Column<'s>> for DatabaseColumn {
    fn from(column: Column<'s>) -> Self {
        let name = column.name().into();
        let decl_type = column.decl_type().and_then(DatabaseDeclType::from_str);
        Self { name, decl_type }
    }
}
