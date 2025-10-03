//! Experimental Options for the Nu codebase.
//!
//! This crate defines all experimental options used in Nushell.
//!
//! An [`ExperimentalOption`] is basically a fancy global boolean.
//! It should be set very early during initialization and lets us switch between old and new
//! behavior for parts of the system.
//!
//! The goal is to have a consistent way to handle experimental flags across the codebase, and to
//! make it easy to find all available options.
//!
//! # Usage
//!
//! Using an option is simple:
//!
//! ```rust
//! if nu_experimental::EXAMPLE.get() {
//!     // new behavior
//! } else {
//!     // old behavior
//! }
//! ```
//!
//! # Adding New Options
//!
//! 1. Create a new module in `options.rs`.
//! 2. Define a marker struct and implement `ExperimentalOptionMarker` for it.
//! 3. Add a new static using `ExperimentalOption::new`.
//! 4. Add the static to [`ALL`].
//!
//! That's it. See [`EXAMPLE`] in `options/example.rs` for a complete example.
//!
//! # For Users
//!
//! Users can view enabled options using either `version` or `debug experimental-options`.
//!
//! To enable or disable options, use either the `NU_EXPERIMENTAL_OPTIONS` environment variable
//! (see [`ENV`]), or pass them via CLI using `--experimental-options`, e.g.:
//!
//! ```sh
//! nu --experimental-options=[example]
//! ```
//!
//! # For Embedders
//!
//! If you're embedding Nushell, prefer using [`parse_env`] or [`parse_iter`] to load options.
//!
//! `parse_iter` is useful if you want to feed in values from other sources.
//! Since options are expected to stay stable during runtime, make sure to do this early.
//!
//! You can also call [`ExperimentalOption::set`] manually, but be careful with that.
//!
//! # Test Support
//!
//! Since experimental options are stored as global statics, testing them can be tricky.
//! Tests running in parallel may interfere with each other by changing the same global value,
//! leading to unstable or flaky results.
//!
//! To make experimental options testable, this crate provides the `test-support` feature.
//! When enabled, it changes how options store their values:
//! instead of using a single global static, each option reads from a
//! [thread-local](std::thread_local) hash map that keeps values isolated per thread.
//!
//! In this mode, the usual setter methods are disabled.
//! Instead, you can use `nu_experimental::test_support::ExperimentalOptionsGuard` to control
//! option values in tests. 
//! The guard allows you to set specific options, and when it’s dropped, all values are 
//! automatically reset.

use std::fmt::Debug;

#[cfg(not(feature = "test-support"))]
use std::sync::atomic::Ordering;

mod options;
pub use options::*;

#[cfg(not(feature = "test-support"))]
mod util;

#[cfg(feature = "test-support")]
pub mod test_support;

#[cfg(not(feature = "test-support"))]
mod parse;
#[cfg(not(feature = "test-support"))]
pub use parse::*;

/// The status of an experimental option.
///
/// An option can either be disabled by default ([`OptIn`](Self::OptIn)) or enabled by default
/// ([`OptOut`](Self::OptOut)), depending on its expected stability.
///
/// Experimental options can be deprecated in two ways:
/// - If the feature becomes default behavior, it's marked as
///   [`DeprecatedDefault`](Self::DeprecatedDefault).
/// - If the feature is being fully removed, it's marked as
///   [`DeprecatedDiscard`](Self::DeprecatedDiscard) and triggers a warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Disabled by default.
    OptIn,
    /// Enabled by default.
    OptOut,
    /// Deprecated as an experimental option; now default behavior.
    DeprecatedDefault,
    /// Deprecated; the feature will be removed and triggers a warning.
    DeprecatedDiscard,
}

/// Experimental option (aka feature flag).
///
/// This struct holds one experimental option that can change some part of Nushell's behavior.
/// These options let users opt in or out of experimental changes while keeping the rest stable.
/// They're useful for testing new ideas and giving users a way to go back to older behavior if needed.
///
/// You can find all options in the statics of [`nu_experimental`](crate).
/// Everything there, except [`ALL`], is a toggleable option.
/// `ALL` gives a full list and can be used to check which options are set.
///
/// The [`Debug`] implementation shows the option's identifier, stability, and current value.
/// To also include the description in the output, use the
/// [plus sign](std::fmt::Formatter::sign_plus), e.g. `format!("{OPTION:+#?}")`.
pub struct ExperimentalOption {
    #[cfg(not(feature = "test-support"))]
    value: crate::util::AtomicMaybe,
    marker: &'static (dyn DynExperimentalOptionMarker + Send + Sync),
}

impl ExperimentalOption {
    /// Construct a new `ExperimentalOption`.
    ///
    /// This should only be used to define a single static for a marker.
    pub(crate) const fn new(
        marker: &'static (dyn DynExperimentalOptionMarker + Send + Sync),
    ) -> Self {
        Self {
            #[cfg(not(feature = "test-support"))]
            value: crate::util::AtomicMaybe::new(None),
            marker,
        }
    }

    pub fn identifier(&self) -> &'static str {
        self.marker.identifier()
    }

    pub fn description(&self) -> &'static str {
        self.marker.description()
    }

    pub fn status(&self) -> Status {
        self.marker.status()
    }

    pub fn since(&self) -> Version {
        self.marker.since()
    }

    pub fn issue_id(&self) -> u32 {
        self.marker.issue()
    }

    pub fn issue_url(&self) -> String {
        format!(
            "https://github.com/nushell/nushell/issues/{}",
            self.marker.issue()
        )
    }

    #[cfg(not(feature = "test-support"))]
    pub fn get(&self) -> bool {
        self.value
            .load(Ordering::Relaxed)
            .unwrap_or_else(|| match self.marker.status() {
                Status::OptIn => false,
                Status::OptOut => true,
                Status::DeprecatedDiscard => false,
                Status::DeprecatedDefault => false,
            })
    }

    /// Sets the state of an experimental option.
    ///
    /// # Safety
    /// This method is unsafe to emphasize that experimental options are not designed to change
    /// dynamically at runtime.
    /// Changing their state at arbitrary points can lead to inconsistent behavior.
    /// You should set experimental options only during initialization, before the application fully
    /// starts.
    #[cfg(not(feature = "test-support"))]
    pub unsafe fn set(&self, value: bool) {
        self.value.store(value, Ordering::Relaxed);
    }

    /// Unsets an experimental option, resetting it to an uninitialized state.
    ///
    /// # Safety
    /// Like [`set`](Self::set), this method is unsafe to highlight that experimental options should
    /// remain stable during runtime.
    /// Only unset options in controlled, initialization contexts to avoid unpredictable behavior.
    #[cfg(not(feature = "test-support"))]
    pub unsafe fn unset(&self) {
        self.value.store(None, Ordering::Relaxed);
    }
}

impl Debug for ExperimentalOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let add_description = f.sign_plus();
        let mut debug_struct = f.debug_struct("ExperimentalOption");
        debug_struct.field("identifier", &self.identifier());
        debug_struct.field("value", &self.get());
        debug_struct.field("status", &self.status());
        if add_description {
            debug_struct.field("description", &self.description());
        }
        debug_struct.finish()
    }
}

impl PartialEq for ExperimentalOption {
    fn eq(&self, other: &Self) -> bool {
        self.identifier() == other.identifier()
    }
}

impl Eq for ExperimentalOption {}

/// Sets the state of all experimental option that aren't deprecated.
///
/// # Safety
/// This method is unsafe to emphasize that experimental options are not designed to change
/// dynamically at runtime.
/// Changing their state at arbitrary points can lead to inconsistent behavior.
/// You should set experimental options only during initialization, before the application fully
/// starts.
#[cfg(not(feature = "test-support"))]
pub unsafe fn set_all(value: bool) {
    for option in ALL {
        match option.status() {
            // SAFETY: The safety bounds for `ExperimentalOption.set` are the same as this function.
            Status::OptIn | Status::OptOut => unsafe { option.set(value) },
            Status::DeprecatedDefault | Status::DeprecatedDiscard => {}
        }
    }
}

pub(crate) trait DynExperimentalOptionMarker {
    fn identifier(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn status(&self) -> Status;
    fn since(&self) -> Version;
    fn issue(&self) -> u32;
}

impl<M: options::ExperimentalOptionMarker> DynExperimentalOptionMarker for M {
    fn identifier(&self) -> &'static str {
        M::IDENTIFIER
    }

    fn description(&self) -> &'static str {
        M::DESCRIPTION
    }

    fn status(&self) -> Status {
        M::STATUS
    }

    fn since(&self) -> Version {
        M::SINCE
    }

    fn issue(&self) -> u32 {
        M::ISSUE
    }
}
