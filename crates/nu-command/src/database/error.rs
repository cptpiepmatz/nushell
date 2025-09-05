use std::path::PathBuf;

use nu_engine::command_prelude::IoError;
use nu_protocol::{ShellError, Span};

pub enum DatabaseError {
    /// Non-database errors.
    /// 
    /// This variant allows easy piping through other errors outside the database context.
    /// The [`Into`] impl for this just extracts.
    Shell(ShellError),

    NotASqliteFile {
        span: Span,
        path: PathBuf,
    }
}

impl From<DatabaseError> for ShellError {
    fn from(value: DatabaseError) -> Self {
        todo!()
    }
}

// Explicitly allow passing through io errors as they nowadays usually provide enough infos.
impl From<IoError> for DatabaseError {
    fn from(error: IoError) -> Self {
        Self::Shell(ShellError::Io(error))
    }
}

#[cfg(test)]
mod assert_no_impl {
    use super::*;

    // ShellError should not be converted automatically to DatabaseError to ensure that all errors 
    // we define in this module are either DatabaseError or passed from another function which 
    // lives outside this module.
    impl From<ShellError> for DatabaseError {
        fn from(_: ShellError) -> Self {
            panic!("ShellError should not be converted automatically to DatabaseError");
        }
    }
}
