use crate::{ClipSet, GetError, SetError, SetReport};
use nu_protocol::{Span, Value};
use std::{error::Error, process::ExitCode};

#[cfg(target_os = "android")]
#[path = "impl/android.rs"]
mod clip_impl;

#[cfg(target_os = "linux")]
#[path = "impl/linux.rs"]
mod clip_impl;

#[cfg(target_os = "macos")]
#[path = "impl/macos.rs"]
mod clip_impl;

#[cfg(target_os = "windows")]
#[path = "impl/windows.rs"]
mod clip_impl;

#[cfg(all(
    not(target_os = "android"),
    not(target_os = "linux"),
    not(target_os = "macos"),
    not(target_os = "windows")
))]
#[path = "impl/other.rs"]
mod clip_impl;

pub use clip_impl::Backend;

pub trait ClipProvider {
    type Error: Error;

    fn set(&self, set: ClipSet) -> Result<SetReport<Self::Error>, SetError<Self::Error>>;

    // get plain text from clipboard
    fn get_text(&self) -> Result<String, GetError>;

    fn get_bytes_from_files(&self) -> Result<Vec<Vec<u8>>, GetError>;

    // get bytes from clipboard via custom type
    fn get_bytes_via_nu(&self) -> Result<Vec<u8>, GetError>;

    // get structured data via nuon deserialization
    fn get_nuon(&self, span: Span) -> Result<Value, GetError>;
}

pub trait ClipServe {
    fn needs_helper(&self, set: &ClipSet) -> bool;

    fn serve(&mut self, set: ClipSet) -> ExitCode;
}
