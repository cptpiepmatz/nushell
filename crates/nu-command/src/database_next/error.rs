use crate::database_next::value::DatabaseStorage;

pub enum DatabaseError {
    OpenConnection {
        storage: DatabaseStorage,
        error: rusqlite::Error,
    }
}