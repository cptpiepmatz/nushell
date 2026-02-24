use std::io;

use chrono::Utc;
use nu_protocol::{ShellError, Value, engine::EngineState};
use nuon::ToNuonConfig;

#[derive(Debug)]
pub struct ClipSet {
    pub(crate) text: Option<String>,
    pub(crate) nuon: Option<String>,
    pub(crate) bytes: Option<(Vec<u8>, String)>,
}

impl ClipSet {
    fn internal_empty() -> ClipSet {
        Self {
            text: None,
            nuon: None,
            bytes: None,
        }
    }

    pub fn empty() -> EmptyClipSet {
        EmptyClipSet
    }

    pub fn with_text(mut self, text: impl ToString) -> Self {
        self.text = Some(text.to_string());
        self
    }

    pub fn with_nuon(
        mut self,
        value: &Value,
        engine_state: &EngineState,
    ) -> Result<Self, ShellError> {
        let config = ToNuonConfig::default();
        let serialized = nuon::to_nuon(engine_state, value, config)?;
        self.nuon = Some(serialized);
        Ok(self)
    }

    pub fn with_bytes(mut self, bytes: impl Into<Vec<u8>>) -> Self {
        self.with_bytes_and_name(
            bytes,
            format!(
                "clipboard_nu_{}.bin",
                Utc::now().format("%Y%m%d_%H%M%S").to_string()
            ),
        )
    }

    pub fn with_bytes_and_name(mut self, bytes: impl Into<Vec<u8>>, name: impl ToString) -> Self {
        self.bytes = Some((bytes.into(), name.to_string()));
        self
    }
}

pub struct EmptyClipSet;

impl EmptyClipSet {
    pub fn with_text(self, text: impl ToString) -> ClipSet {
        ClipSet::internal_empty().with_text(text)
    }

    pub fn with_nuon(
        mut self,
        value: &Value,
        engine_state: &EngineState,
    ) -> Result<ClipSet, ShellError> {
        ClipSet::internal_empty().with_nuon(value, engine_state)
    }

    pub fn with_bytes(mut self, bytes: impl Into<Vec<u8>>) -> ClipSet {
        ClipSet::internal_empty().with_bytes(bytes)
    }

    pub fn with_bytes_and_name(
        mut self,
        bytes: impl Into<Vec<u8>>,
        name: impl ToString,
    ) -> ClipSet {
        ClipSet::internal_empty().with_bytes_and_name(bytes, name)
    }
}

#[non_exhaustive]
#[must_use]
pub struct SetReport<BE> {
    pub text: SetStatus<BE>,
    pub nuon: SetStatus<BE>,
    pub bytes_nu: SetStatus<BE>,
    pub bytes_file: SetStatus<BE>,
}

impl<BE> SetReport<BE> {
    pub fn any_ok(self) -> Result<(), Vec<SetError<BE>>> {
        let mut errors = Vec::new();
        for status in [self.text, self.nuon, self.bytes_nu, self.bytes_file] {
            match status {
                SetStatus::NotRequested => continue,
                SetStatus::Set => return Ok(()),
                SetStatus::Failed(set_error) => errors.push(set_error),
            }
        }

        debug_assert!(
            errors.is_empty(),
            "a report should never have only not requested fields"
        );
        Err(errors)
    }

    pub fn all_ok(self) -> Result<(), Vec<SetError<BE>>> {
        let errors: Vec<_> = [self.text, self.nuon, self.bytes_nu, self.bytes_file]
            .into_iter()
            .filter_map(|status| match status {
                SetStatus::NotRequested | SetStatus::Set => None,
                SetStatus::Failed(set_error) => Some(set_error),
            })
            .collect();

        match errors.is_empty() {
            true => Ok(()),
            false => Err(errors),
        }
    }
}

#[derive(Debug)]
pub enum SetStatus<BE> {
    NotRequested,
    Set,
    Failed(SetError<BE>),
}

#[derive(Debug)]
pub enum SetError<BE> {
    Setup(BE),
    Io(io::Error),
    Other(BE),
    // relevant errors when setting the clipboard
}

#[derive(Debug)]
pub enum GetError {
    // relevant errors when getting the clipboard
}
