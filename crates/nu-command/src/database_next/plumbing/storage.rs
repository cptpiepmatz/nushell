use std::path::Path;

use nu_path::AbsolutePathBuf;
use nu_protocol::Span;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabaseStorage {
    File { path: AbsolutePathBuf, span: Span },
    InMemoryStor { span: Span },
    InMemoryHistory,
}

impl DatabaseStorage {
    /// Get storage path for the database.
    ///
    /// The return is marked as a [`Path`] as [`Connection::open`](rusqlite::Connection::open) asks
    /// for an [`AsRef<Path>`](AsRef) even though this might contain in memory values like
    /// ":memory:".
    pub fn as_path(&self) -> &Path {
        match self {
            DatabaseStorage::File { path, .. } => path.as_std_path(),
            DatabaseStorage::InMemoryStor { .. } => Path::new(":memory:"),
            DatabaseStorage::InMemoryHistory => Path::new("file:memdb1?mode=memory&cache=shared"),
        }
    }
}
