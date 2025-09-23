use std::{
    hash::{BuildHasher, Hash, Hasher, RandomState},
    path::Path,
    sync::LazyLock,
};

use nu_path::AbsolutePath;
use nu_protocol::Span;
use rusqlite::OpenFlags;
use serde::{Deserialize, Serialize};

use crate::database_next::plumbing::uri::DatabaseUri;

/// Process local deterministic ID hasher.
///
/// Provides a process-local hasher for deterministic, reproducible ids.
/// The same input will always hash to the same value within a single run.
/// Not stable across different runs or binaries.
static RANDOM_STATE: LazyLock<RandomState> = LazyLock::new(RandomState::new);

static STOR_URI: LazyLock<DatabaseUri> =
    LazyLock::new(|| DatabaseUri::new("", "memory", [] as [(&str, &str); 0]));
static HISTORY_URI: LazyLock<DatabaseUri> =
    LazyLock::new(|| DatabaseUri::new("file", "memdb1", [("mode", "memory"), ("cache", "shared")]));

/// Storage location and access mode for a SQLite database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabaseStorage {
    /// File on the local filesystem used for read-only operations.
    ///
    /// We run queries directly on the file so the OS only reads pages we touch.
    /// Good for big DBs and read-heavy work.
    ReadonlyFile { path: DatabaseUri, span: Span },

    /// Writable named in-memory database.
    ///
    /// All connections opened with the same address share the same DB.
    /// This can be created by promoting a `ReadonlyFile` or by loading from raw bytes.
    WritableMemory { path: DatabaseUri, span: Span },

    /// Ephemeral in-memory DB for `stor` commands.
    InMemoryStor { span: Span },

    /// Named in-memory DB for history when backed by SQLite.
    InMemoryHistory,
}

impl DatabaseStorage {
    pub fn new_readonly_file(path: &AbsolutePath, span: Span) -> Self {
        let path = DatabaseUri::new("file", path, [("mode", "ro"), ("immutable", "1")]);
        Self::ReadonlyFile { path, span }
    }

    pub fn new_writable_memory(id: impl Hash, span: Span) -> Self {
        let mut hasher = RANDOM_STATE.build_hasher();
        id.hash(&mut hasher);
        let id = hasher.finish();

        let path = DatabaseUri::new(
            "file",
            format!("nu-sqlite-{id:016x}"),
            [("mode", "memory"), ("cache", "shared")],
        );
        Self::WritableMemory { path, span }
    }

    /// Get storage path for the database.
    ///
    /// The return is marked as a [`Path`] as [`Connection::open`](rusqlite::Connection::open) asks
    /// for an [`AsRef<Path>`](AsRef) even though this might contain in memory values like
    /// ":memory:".
    pub fn connection_path(&self) -> &Path {
        match self {
            Self::ReadonlyFile { path, .. } => path.uri(),
            Self::WritableMemory { path, .. } => path.uri(),
            Self::InMemoryStor { .. } => STOR_URI.uri(),
            Self::InMemoryHistory => HISTORY_URI.uri(),
        }
    }

    pub fn path(&self) -> &Path {
        match self {
            Self::ReadonlyFile { path, .. } => path.path(),
            Self::WritableMemory { path, .. } => path.path(),
            Self::InMemoryStor { .. } => STOR_URI.path(),
            Self::InMemoryHistory => HISTORY_URI.path(),
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
