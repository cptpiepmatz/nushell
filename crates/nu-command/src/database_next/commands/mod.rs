use nu_protocol::engine::StateWorkingSet;

mod from_db;
mod from_sqlite;
mod query_db;
mod schema;
mod to_db;
mod to_sqlite;

pub use from_db::*;
pub use from_sqlite::*;
pub use query_db::*;
pub use schema::*;
pub use to_db::*;
pub use to_sqlite::*;

pub fn add_database_decls(working_set: &mut StateWorkingSet) {
    working_set.add_decl(Box::new(FromSqlite));
}