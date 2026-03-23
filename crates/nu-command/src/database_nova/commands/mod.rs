use nu_protocol::engine::StateWorkingSet;

mod from;
mod query_db;
mod schema;
mod to;

pub use from::*;
pub use query_db::*;
pub use schema::*;
pub use to::*;

pub fn add_database_decls(working_set: &mut StateWorkingSet) {
    working_set.add_decl(Box::new(FROM_SQLITE));
    working_set.add_decl(Box::new(FROM_DB));
    working_set.add_decl(Box::new(TO_SQLITE));
    working_set.add_decl(Box::new(TO_DB));
}
