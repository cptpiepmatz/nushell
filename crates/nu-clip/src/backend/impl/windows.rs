use std::{
    env, ffi::CStr, fs, io, iter, os::windows::ffi::OsStrExt, path::PathBuf, ptr, sync::LazyLock,
};

use const_format::formatcp;
use nu_protocol::{Span, Value, engine::EngineState};
use windows::{
    Win32::{
        Foundation::{GetLastError, GlobalFree, HANDLE, HWND, POINT},
        System::{
            Console::GetConsoleWindow,
            DataExchange::{
                CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatA,
                SetClipboardData,
            },
            LibraryLoader::GetModuleHandleA,
            Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock},
            Ole::{CF_HDROP, CF_UNICODETEXT},
        },
        UI::{
            Shell::DROPFILES,
            WindowsAndMessaging::{
                CW_USEDEFAULT, CreateWindowExA, DestroyWindow, HWND_MESSAGE, WINDOW_EX_STYLE,
                WINDOW_STYLE,
            },
        },
    },
    core::{PCSTR, s},
};

use crate::{
    ClipSet, GetError, SetError, SetReport, SetStatus,
    backend::{ClipProvider, ClipServe},
};

/// Nuon format name as null-terminated ansi string.
static NUON_FORMAT_NAME: &[u8] = formatcp!("nushell.nuon.v{}\0", nuon::VERSION).as_bytes();
const _: () = assert!(NUON_FORMAT_NAME.is_ascii()); // valid ascii is also valid ansi
const _: () = assert!(CStr::from_bytes_with_nul(NUON_FORMAT_NAME).is_ok()); // only terminating null

/// Clipboard format registration ID for nuon format.
static NUON_FORMAT: LazyLock<u32> = LazyLock::new(|| {
    // SAFETY: pointer to a valid null-terminated ansi string
    let id = unsafe { RegisterClipboardFormatA(PCSTR(NUON_FORMAT_NAME.as_ptr())) };
    debug_assert_ne!(id, 0, "registering nuon clipboard format failed");
    id
});

static NUON_MAGIC: &str = "NUON";

/// Raw bytes format name as null-terminated ansi string.
static RAW_BYTES_FORMAT_NAME: &[u8] =
    formatcp!("nushell.bytes.v{}\0", env!("CARGO_PKG_VERSION")).as_bytes();
const _: () = assert!(RAW_BYTES_FORMAT_NAME.is_ascii());
const _: () = assert!(CStr::from_bytes_with_nul(RAW_BYTES_FORMAT_NAME).is_ok());

/// Clipboard format registration ID for raw bytes format.
static RAW_BYTES_FORMAT: LazyLock<u32> = LazyLock::new(|| {
    // SAFETY: pointer to a valid null-terminated ansi string
    let id = unsafe { RegisterClipboardFormatA(PCSTR(RAW_BYTES_FORMAT_NAME.as_ptr())) };
    debug_assert_ne!(id, 0, "registering raw bytes clipboard format failed");
    id
});

static RAW_BYTES_MAGIC: &str = "NU_RAW_BYTES";

static TEMP_DIR_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| env::temp_dir().join("nushell").join("clipboard"));

enum ClipboardWindow {
    Console(HWND),
    Owned(HWND),
}

impl ClipboardWindow {
    // TODO: work with real error type here
    fn create() -> Result<Self, windows::core::Error> {
        // SAFETY: using Rust on Windows targets always higher than _WIN32_WINNT,
        //         we also verify that the handle is not null
        let console_hwnd = unsafe { GetConsoleWindow() };
        if !console_hwnd.is_invalid() {
            return Ok(Self::Console(console_hwnd));
        }

        // SAFETY: by passing null, we get the app itself, that should never fail, also this code 
        //         is not called using LOAD_LIBRARY_AS_DATAFILE
        let app_module = unsafe { GetModuleHandleA(PCSTR::null()) }
            .expect("null always returns valid module")
            .into();
        // SAFETY: - class/title are null-terminated static strings from `s!(...)`
        //         - `app_module` is a valid HINSTANCE from `GetModuleHandleA(null)`
        //         - mesage-only "STATIC" window does not require a custom WndProc
        //         - `lpParam` may be null
        let owned_hwnd = unsafe {
            CreateWindowExA(
                WINDOW_EX_STYLE(0),
                s!("STATIC"),
                s!("nu-clipboard-handler"),
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                Some(app_module),
                None,
            )
        }?;

        Ok(Self::Owned(owned_hwnd))
    }

    fn window_handle(&self) -> HWND {
        match self {
            ClipboardWindow::Console(hwnd) => *hwnd,
            ClipboardWindow::Owned(hwnd) => *hwnd,
        }
    }
}

impl Drop for ClipboardWindow {
    fn drop(&mut self) {
        if let Self::Owned(window_handle) = self {
            let destroyed = unsafe { DestroyWindow(*window_handle) };
            debug_assert!(
                destroyed.is_ok(),
                "could not destroy owned clipboard window"
            );
        }
    }
}

struct Clipboard {
    window: ClipboardWindow,
}

impl Clipboard {
    fn open() -> Result<Self, windows::core::Error> {
        let window = ClipboardWindow::create()?;
        let window_handle = window.window_handle();
        // SAFETY: we pass a valid window handle and we're closing the clipboard again after a 
        //         successful open via the `Drop` impl
        unsafe { OpenClipboard(Some(window_handle)) }?;
        Ok(Clipboard { window })
    }

    fn set(
        &self,
        set: ClipSet,
    ) -> Result<SetReport<windows::core::Error>, SetError<windows::core::Error>> {
        unsafe { EmptyClipboard() }.map_err(SetError::Setup)?;

        if let Some(text) = set.text {
            let utf16_chars: Vec<_> = text.encode_utf16().chain(iter::once(0)).collect();
            let size_bytes = utf16_chars.len() * size_of::<u16>();
            unsafe {
                let global_handle = GlobalAlloc(GMEM_MOVEABLE, size_bytes).unwrap();
                if global_handle.is_invalid() {
                    todo!();
                }

                let global_lock = GlobalLock(global_handle) as *mut u8;
                if global_lock.is_null() {
                    todo!();
                }

                ptr::copy_nonoverlapping(
                    utf16_chars.as_ptr() as *const u8,
                    global_lock,
                    size_bytes,
                );

                let unlock = GlobalUnlock(global_handle);
                if unlock.is_err() {
                    let last_err = io::Error::last_os_error();
                    if let Some(0) = last_err.raw_os_error() {
                    } else {
                        todo!();
                    }
                }

                let set_data =
                    SetClipboardData(CF_UNICODETEXT.0.into(), Some(HANDLE(global_handle.0)));
                if set_data.is_err() {
                    todo!()
                }
            }
        }

        if let Some(nuon) = set.nuon {
            let magic = NUON_MAGIC.as_bytes();
            let len = nuon.len().to_ne_bytes();
            let nuon_bytes = nuon.as_bytes();
            let payload_bytes = magic.len() + len.len() + nuon_bytes.len();
            unsafe {
                let global_handle = GlobalAlloc(GMEM_MOVEABLE, payload_bytes).unwrap();
                if global_handle.is_invalid() {
                    todo!();
                }

                let global_lock = GlobalLock(global_handle) as *mut u8;
                if global_lock.is_null() {
                    todo!();
                }

                for (offset, byte) in magic
                    .iter()
                    .chain(len.iter())
                    .chain(nuon_bytes.iter())
                    .enumerate()
                {
                    unsafe {
                        global_lock.add(offset).write(*byte);
                    }
                }

                let unlock = GlobalUnlock(global_handle);
                if unlock.is_err() {
                    let last_err = io::Error::last_os_error();
                    if let Some(0) = last_err.raw_os_error() {
                    } else {
                        todo!();
                    }
                }

                let set_data = SetClipboardData(*NUON_FORMAT, Some(HANDLE(global_handle.0)));
                if set_data.is_err() {
                    todo!()
                }
            }
        }

        if let Some((bytes, name)) = set.bytes {
            let magic = RAW_BYTES_MAGIC.as_bytes();
            let len = bytes.len().to_ne_bytes();
            let payload_bytes = magic.len() + len.len() + bytes.len();
            unsafe {
                let global_handle = GlobalAlloc(GMEM_MOVEABLE, payload_bytes).unwrap();
                if global_handle.is_invalid() {
                    todo!();
                }

                let global_lock = GlobalLock(global_handle) as *mut u8;
                if global_lock.is_null() {
                    todo!();
                }

                for (offset, byte) in magic
                    .iter()
                    .chain(len.iter())
                    .chain(bytes.iter())
                    .enumerate()
                {
                    unsafe {
                        global_lock.add(offset).write(*byte);
                    }
                }

                let unlock = GlobalUnlock(global_handle);
                if unlock.is_err() {
                    let last_err = io::Error::last_os_error();
                    if let Some(0) = last_err.raw_os_error() {
                    } else {
                        todo!();
                    }
                }

                let set_data = SetClipboardData(*RAW_BYTES_FORMAT, Some(HANDLE(global_handle.0)));
                if set_data.is_err() {
                    todo!()
                }
            }

            fs::create_dir_all(TEMP_DIR_PATH.as_path()).unwrap();
            let file_path = TEMP_DIR_PATH.join(name);
            fs::write(&file_path, bytes).unwrap();

            let dropfiles_size = size_of::<DROPFILES>();
            let dropfiles = DROPFILES {
                pFiles: dropfiles_size as u32,
                pt: POINT { x: 0, y: 0 },
                fNC: false.into(),
                fWide: true.into(),
            };
            let file_paths: Vec<u16> = file_path.as_os_str().encode_wide().chain([0, 0]).collect();
            let file_paths_size = file_paths.len() * size_of::<u16>();
            let payload_size = dropfiles_size + file_paths_size;
            unsafe {
                let global_handle = GlobalAlloc(GMEM_MOVEABLE, payload_size).unwrap();
                if global_handle.is_invalid() {
                    todo!();
                }

                let global_lock = GlobalLock(global_handle) as *mut u8;
                if global_lock.is_null() {
                    todo!();
                }

                unsafe {
                    ptr::copy_nonoverlapping(&dropfiles, global_lock as *mut DROPFILES, 1);
                    ptr::copy_nonoverlapping(
                        file_paths.as_ptr() as *const u8,
                        global_lock.add(dropfiles_size),
                        file_paths_size,
                    );
                }

                let unlock = GlobalUnlock(global_handle);
                if unlock.is_err() {
                    let last_err = io::Error::last_os_error();
                    if let Some(0) = last_err.raw_os_error() {
                    } else {
                        todo!();
                    }
                }

                let set_data = SetClipboardData(CF_HDROP.0.into(), Some(HANDLE(global_handle.0)));
                if set_data.is_err() {
                    todo!()
                }
            }
        }

        Ok(SetReport {
            text: SetStatus::Set,
            nuon: SetStatus::Set,
            bytes_nu: SetStatus::Set,
            bytes_file: SetStatus::Set,
        })
    }
}

#[test]
fn test_clipboard() {
    let engine_state = EngineState::new();

    let value_str = r#"[[name, type, size, modified]; [".VirtualBox", dir, 4096b, 2025-07-02T22:59:33.920067100+02:00], [".affinity", dir, 0b, 2024-11-14T21:55:06.624472400+01:00], [".android", dir, 4096b, 2026-02-21T18:02:29.861206100+01:00], [".angular-config.json", file, 142b, 2025-02-23T18:17:31.784239+01:00], [".aws", dir, 0b, 2024-11-21T16:55:54.175679300+01:00], [".azure", dir, 0b, 2024-11-21T16:55:54.504731600+01:00], [".bash_history", file, 91b, 2025-10-09T22:13:14.144733400+02:00], [".blackbox", dir, 0b, 2025-09-29T21:28:58.183733600+02:00], [".bun", dir, 0b, 2025-06-19T17:35:46.045631600+02:00], [".cache", dir, 4096b, 2025-12-01T16:50:43.235354100+01:00], [".config", dir, 4096b, 2025-10-16T14:32:03.960370700+02:00], [".continue", dir, 4096b, 2025-01-31T17:48:14.026344300+01:00], [".cortex-debug", file, 47b, 2024-11-14T21:42:32.903790400+01:00], [".docker", dir, 4096b, 2026-02-12T14:50:09.371224800+01:00], [".dotnet", dir, 8192b, 2025-08-12T09:18:22.929089300+02:00], [".emulator_console_auth_token", file, 16b, 2025-10-09T21:03:45.061972+02:00], [".g8", dir, 0b, 2026-01-18T13:30:04.258688600+01:00], [".gitconfig", file, 660b, 2025-10-26T09:28:02.864750200+01:00], [".gradle", dir, 4096b, 2025-07-16T18:00:16.913062900+02:00], [".ipython", dir, 0b, 2024-11-20T17:09:06.159457600+01:00], [".ivy2", dir, 4096b, 2026-01-24T12:17:28.569664600+01:00], [".jupyter", dir, 0b, 2024-11-29T18:25:46.391223100+01:00], [".lesshst", file, 20b, 2025-10-25T13:39:32.219157500+02:00], [".local", dir, 0b, 2025-06-02T16:35:18.698037400+02:00], [".matplotlib", dir, 0b, 2024-11-22T16:55:13.614061300+01:00], [".metals", dir, 0b, 2025-07-03T11:56:47.505699800+02:00], [".motion-canvas", dir, 0b, 2025-01-21T19:58:33.671597900+01:00], [".node_repl_history", file, 1029b, 2025-12-10T16:17:19.455343200+01:00], [".nu", dir, 0b, 2024-11-17T12:24:22.008870600+01:00], [".nuget", dir, 0b, 2025-02-06T16:52:07.074588400+01:00], [".ollama", dir, 4096b, 2025-01-23T01:00:14.852986700+01:00], [".openjfx", dir, 0b, 2024-11-25T15:04:18.039331500+01:00], [".sbt", dir, 0b, 2026-01-18T13:28:52.805678500+01:00], [".ssh", dir, 4096b, 2026-01-28T10:07:06.409064900+01:00], [".streamlit", dir, 0b, 2025-10-27T13:00:38.068871400+01:00], [".templateengine", dir, 0b, 2025-02-06T16:47:54.145183200+01:00], [".th-client", dir, 0b, 2025-02-02T20:44:22.835537300+01:00], [".vivaldi", dir, 0b, 2024-08-09T09:49:24.444242900+02:00], [".vivaldi.zip", file, 566b, 2025-05-18T16:31:59.072557400+02:00], [".vivaldi_reporting_data", file, 527b, 2026-02-23T17:58:30.129007600+01:00], [".vscode", dir, 0b, 2024-11-14T21:39:45.777492800+01:00], [".vscode-server", dir, 4096b, 2025-07-29T10:01:04.103187800+02:00], [".wslconfig", file, 67b, 2025-05-23T01:28:11.142865200+02:00], [".xargo", dir, 0b, 2025-01-15T09:19:04.974469100+01:00], ["4diacIDE-workspace", dir, 0b, 2025-08-11T10:29:32.149869600+02:00], [Anwendungsdaten, symlink, 0b, 2025-10-26T19:47:49.296781300+01:00], [Contacts, dir, 0b, 2025-10-27T10:00:03.918463100+01:00], [CrossDevice, dir, 0b, 2024-12-12T09:52:42.315831+01:00], [Desktop, dir, 4096b, 2026-02-10T16:04:25.280844900+01:00], [Documents, dir, 12288b, 2026-02-05T00:06:48.666738900+01:00], [Druckumgebung, symlink, 0b, 2025-10-26T19:47:49.298290+01:00], ["Eigene Dateien", symlink, 0b, 2025-10-26T19:47:49.293645800+01:00], [Favorites, dir, 0b, 2025-10-27T10:00:03.923803200+01:00], ["Google Drive-Streaming", dir, 0b, 2024-11-14T16:12:46.951896500+01:00], [Links, dir, 0b, 2025-10-27T10:00:04.192685600+01:00], ["Lokale Einstellungen", symlink, 0b, 2025-10-26T19:47:49.301415600+01:00], [Music, dir, 0b, 2025-10-27T10:00:04.008534300+01:00], [Netzwerkumgebung, symlink, 0b, 2025-10-26T19:47:49.297786400+01:00], [OneDrive, dir, 0b, 2025-10-27T10:02:32.677903600+01:00], [Pictures, dir, 8192b, 2026-01-16T14:41:03.815175800+01:00], [Recent, symlink, 0b, 2025-10-26T19:47:49.298290+01:00], ["Saved Games", dir, 4096b, 2025-10-27T10:00:04.118818600+01:00], [Searches, dir, 4096b, 2025-12-22T22:52:45.316806400+01:00], [SendTo, symlink, 0b, 2025-10-26T19:47:49.298290+01:00], [Startmenü, symlink, 0b, 2025-10-26T19:47:49.299846500+01:00], [Videos, dir, 8192b, 2026-02-23T23:50:12.246488200+01:00], [Vorlagen, symlink, 0b, 2025-10-26T19:47:49.299846500+01:00], [_lesshst, file, 21b, 2026-01-20T17:50:58.448723200+01:00], ["a.db", file, 8192b, 2025-09-05T16:11:47.872605400+02:00], [ansel, dir, 0b, 2024-11-14T15:30:04.796419600+01:00], [bin, dir, 4096b, 2025-10-08T17:52:28.663654400+02:00], [clipboard, file, 2025b, 2025-07-07T09:28:54.154432200+02:00], [deploy_key, file, 399b, 2025-02-27T23:02:07.275400+01:00], ["deploy_key.pub", file, 93b, 2025-02-27T23:02:07.276400100+01:00], ["derPi.zip", file, 18273800664b, 2024-11-14T13:20:44.888876600+01:00], [emu, dir, 0b, 2026-01-10T01:41:07.626581900+01:00], ["expose-notes.md", file, 1428b, 2024-12-09T16:20:52.197137800+01:00], [file, file, 3246b, 2025-03-17T12:48:28.687311700+01:00], [go, dir, 0b, 2025-06-11T17:41:07.386628500+02:00], ["secrets.nuon", file, 655b, 2025-11-17T13:14:36.100640200+01:00], ["shot.ansi", file, 2754b, 2025-09-01T14:19:42.461144500+02:00], [some_file, file, 5b, 2025-06-03T21:26:27.234277200+02:00], [some_link, file, 5b, 2025-06-03T21:26:27.234277200+02:00], [source, dir, 0b, 2025-08-18T19:02:57.566274300+02:00], [tmp, dir, 0b, 2025-08-27T20:30:32.935855600+02:00], ["wingets.json", file, 6394b, 2024-11-14T13:13:39.142107900+01:00]]"#;
    let value = nuon::from_nuon(value_str, None).unwrap();

    let clipboard = Clipboard::open().unwrap();
    clipboard.set(
        ClipSet::empty()
            .with_text("something")
            .with_nuon(&value, &engine_state)
            .unwrap()
            .with_bytes(value_str),
    );
}

impl Drop for Clipboard {
    fn drop(&mut self) {
        // SAFETY: only accessible after after a successful `open`, Windows ensures that only one
        //         accessor exists at the same time
        let close = unsafe { CloseClipboard() };
        debug_assert!(close.is_ok(), "could not close clipboard");
    }
}

pub struct Backend;

impl ClipProvider for Backend {
    type Error = windows::core::Error;

    fn set(&self, set: ClipSet) -> Result<SetReport<Self::Error>, SetError<Self::Error>> {
        todo!()
    }

    fn get_text(&self) -> Result<String, GetError> {
        todo!()
    }

    fn get_bytes_from_files(&self) -> Result<Vec<Vec<u8>>, GetError> {
        todo!()
    }

    fn get_bytes_via_nu(&self) -> Result<Vec<u8>, GetError> {
        todo!()
    }

    fn get_nuon(&self, span: nu_protocol::Span) -> Result<nu_protocol::Value, GetError> {
        todo!()
    }
}

impl ClipServe for Backend {
    fn needs_helper(&self, _: &ClipSet) -> bool {
        false
    }

    fn serve(&mut self, _: ClipSet) -> std::process::ExitCode {
        unimplemented!("not needed")
    }
}
