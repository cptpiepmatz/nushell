use crate::{
    ClipSet, SetError, SetReport, SetStatus,
    backend::{Backend, ClipProvider},
};

pub type BackendError = <Backend as ClipProvider>::Error;

pub struct Clipboard {
    backend: Backend,
}

impl Clipboard {
    pub fn set(set: ClipSet) -> SetReport<BackendError> {
        todo!()
    }

    pub fn set_text(text: impl ToString) -> Result<(), SetError<BackendError>> {
        let report = Self::set(ClipSet::empty().with_text(text));
        match report.text {
            SetStatus::NotRequested => unreachable!("requested text"),
            SetStatus::Set => Ok(()),
            SetStatus::Failed(set_error) => Err(set_error),
        }
    }
}
