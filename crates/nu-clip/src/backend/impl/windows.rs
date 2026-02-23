use std::{ffi::CStr, sync::LazyLock};

use const_format::formatcp;
use windows::{Win32::System::DataExchange::RegisterClipboardFormatA, core::PCSTR};

use crate::backend::{ClipProvider, ClipServe};

/// Null-terminated ansi string.
static NUON_FORMAT_NAME: &[u8] = formatcp!("nushell.nuon.v{}\0", nuon::VERSION).as_bytes();
const _: () = assert!(NUON_FORMAT_NAME.is_ascii()); // valid ascii is also valid ansi
const _: () = assert!(CStr::from_bytes_with_nul(NUON_FORMAT_NAME).is_ok()); // only terminating null

/// Clipboard format registration ID for nuon format.
static NUON_FORMAT: LazyLock<u32> = LazyLock::new(|| {
    // SAFETY: pointer to a valid null-terminated ansi string
    let id = unsafe { RegisterClipboardFormatA(PCSTR(NUON_FORMAT_NAME.as_ptr())) };
    debug_assert_ne!(id, 0, "registering clipboard format failed");
    id
});

pub struct Backend;

impl ClipProvider for Backend {
    fn set(&self, set: crate::ClipSet) -> crate::SetReport {
        todo!()
    }

    fn get_text(&self) -> Result<String, crate::GetError> {
        todo!()
    }

    fn get_bytes_from_files(&self) -> Result<Vec<Vec<u8>>, crate::GetError> {
        todo!()
    }

    fn get_bytes_via_nu(&self) -> Result<Vec<u8>, crate::GetError> {
        todo!()
    }

    fn get_nuon(&self, span: nu_protocol::Span) -> Result<nu_protocol::Value, crate::GetError> {
        todo!()
    }
}

impl ClipServe for Backend {
    fn needs_helper(&self, set: &crate::ClipSet) -> bool {
        false
    }

    fn serve(&mut self, set: crate::ClipSet) -> std::process::ExitCode {
        unimplemented!("not needed")
    }
}
