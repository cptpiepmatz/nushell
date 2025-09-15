use std::{
    hash::{BuildHasher, Hash, Hasher, RandomState},
    path::Path,
    sync::LazyLock,
};

use nu_path::AbsolutePathBuf;
use nu_protocol::Span;
use rusqlite::OpenFlags;
use serde::{Deserialize, Serialize};

/// Process local deterministic ID hasher.
///
/// Provides a process-local hasher for deterministic, reproducible ids.
/// The same input will always hash to the same value within a single run.
/// Not stable across different runs or binaries.
static RANDOM_STATE: LazyLock<RandomState> = LazyLock::new(RandomState::new);

/// Storage location and access mode for a SQLite database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabaseStorage {
    /// File on the local filesystem used for read-only operations.
    ///
    /// We run queries directly on the file so the OS only reads pages we touch.
    /// Good for big DBs and read-heavy work.
    ReadonlyFile { path: AbsolutePathBuf, span: Span },

    /// Writable named in-memory database.
    ///
    /// `address` should be a SQLite memory URI like:
    /// `file:<id>?mode=memory&cache=shared`
    ///
    /// All connections opened with the same address share the same DB.
    /// This can be created by promoting a `ReadonlyFile` or by loading from raw bytes.
    WritableMemory { address: String, span: Span },

    /// Ephemeral in-memory DB for `stor` commands.
    InMemoryStor { span: Span },

    /// Named in-memory DB for history when backed by SQLite.
    InMemoryHistory,
}

impl DatabaseStorage {
    pub fn new_writable_memory(id: impl Hash, span: Span) -> Self {
        let mut hasher = RANDOM_STATE.build_hasher();
        id.hash(&mut hasher);
        let id = hasher.finish();

        let address = format!("file:nu-sqlite-{id:016x}?mode=memory&cache=shared");
        Self::WritableMemory { address, span }
    }

    /// Get storage path for the database.
    ///
    /// The return is marked as a [`Path`] as [`Connection::open`](rusqlite::Connection::open) asks
    /// for an [`AsRef<Path>`](AsRef) even though this might contain in memory values like
    /// ":memory:".
    pub fn as_path(&self) -> &Path {
        match self {
            Self::ReadonlyFile { path, .. } => path.as_std_path(),
            Self::WritableMemory { address, .. } => Path::new(address),
            Self::InMemoryStor { .. } => Path::new(":memory:"),
            Self::InMemoryHistory => Path::new("file:memdb1?mode=memory&cache=shared"),
        }
    }

    pub fn flags(&self) -> OpenFlags {
        match self {
            Self::WritableMemory { .. } | Self::InMemoryStor { .. } | Self::InMemoryHistory => {
                OpenFlags::default()
            }
            Self::ReadonlyFile { .. } => {
                OpenFlags::SQLITE_OPEN_READ_ONLY
                    | OpenFlags::SQLITE_OPEN_URI
                    | OpenFlags::SQLITE_OPEN_NO_MUTEX
                    | OpenFlags::SQLITE_OPEN_PRIVATE_CACHE
            }
        }
    }
}
