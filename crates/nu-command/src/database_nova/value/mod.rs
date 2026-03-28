use crate::database_nova::plumbing::connection::DatabaseConnection;
use nu_protocol::{ShellError, Span, Spanned, shell_error::io::IoError};
use std::{
    fs,
    ops::Deref,
    path::{Path, PathBuf},
};

mod database;
mod database_system;
mod database_table;

pub use database::*;
pub use database_system::*;
pub use database_table::*;

fn save_database_value(
    conn: &DatabaseConnection,
    path: Spanned<&Path>,
    value_span: Span,
    save_span: Span,
) -> Result<(), ShellError> {
    let bytes = conn.serialize(value_span, save_span)?;
    let bytes = bytes.deref();
    let Spanned {
        item: path,
        span: path_span,
    } = path;
    fs::write(&path, bytes).map_err(|error| IoError::new(error, path_span, PathBuf::from(path)))?;
    Ok(())
}
