use std::{cell::RefCell, collections::HashMap};

use super::*;

thread_local! {
    static VALUES: RefCell<HashMap<&'static str, bool>> = Default::default();
}

pub struct ExperimentalOptionsGuard;

impl ExperimentalOptionsGuard {
    pub fn get() -> Self {
        Self
    }

    pub fn set(&mut self, option: &'static ExperimentalOption, value: bool) {
        VALUES.with_borrow_mut(|values| {
            values.insert(option.identifier(), value);
        });
    }
}

impl ExperimentalOption {
    pub fn get(&self) -> bool {
        VALUES.with_borrow(|values| {
            values
                .get(self.identifier())
                .cloned()
                .unwrap_or_else(|| match self.marker.status() {
                    Status::OptIn => false,
                    Status::OptOut => true,
                    Status::DeprecatedDiscard => false,
                    Status::DeprecatedDefault => false,
                })
        })
    }
}

impl Drop for ExperimentalOptionsGuard {
    fn drop(&mut self) {
        VALUES.with_borrow_mut(|values| values.clear());
    }
}
