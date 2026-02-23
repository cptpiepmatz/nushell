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
pub struct SetReport {
    pub text: SetStatus,
    pub nuon: SetStatus,
    pub bytes_nu: SetStatus,
    pub bytes_file: SetStatus,
}

#[derive(Debug, Clone)]
pub enum SetStatus {
    NotRequested,
    Set,
    Failed(SetError),
}

#[derive(Debug, Clone)]
pub enum SetError {
    // relevant errors when setting the clipboard
}

#[derive(Debug, Clone)]
pub enum GetError {
    // relevant errors when getting the clipboard
}
