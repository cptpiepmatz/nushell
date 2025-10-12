#![allow(
    deprecated,
    reason = "We use deprecation warnings to document that manual construction is not allowed."
)]

use std::{fmt::Debug, ops::Deref, sync::LazyLock};

use crate::{self as nu_test_support, harness::output_capture::Output};

use libtest_mimic::{Arguments, Trial};
#[doc(hidden)]
pub use linkme;

pub mod output_capture;
pub mod macros {
    pub use linkme::distributed_slice as collect_test;
    pub use nu_test_support_macros::test;
}

// generate data types for `TestMetadata`
nu_test_support_macros::make_metadata!();

/// All collected tests.
#[linkme::distributed_slice]
#[linkme(crate = nu_test_support::harness::linkme)]
pub static TESTS: [TestMetadata];

/// A test function returning an arbitrary error.
pub type TestFn = fn() -> Result<(), Box<dyn std::error::Error>>;

/// Metadata of a test, including the pointer to it.
pub struct TestMetadata {
    /// The actual test function.
    pub function: TestFn,
    /// The full name of test according to [`std::any::type_name`].
    pub name: LazyLock<&'static str>,
    /// Whether the test is ignored and its reason.
    pub ignored: (bool, Option<&'static str>),
    /// Whether the test should panic and what is expected.
    pub should_panic: (bool, Option<&'static str>),
    /// Requested experimental options.
    ///
    /// The type is generated from [`nu_experimental::ALL`].
    pub experimental_options: RequestedExperimentalOptions,
}

impl Debug for TestMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestMetadata")
            .field("function", &self.function)
            .field("name", self.name.deref())
            .field("ignored", &self.ignored)
            .field("should_panic", &self.should_panic)
            .field("experimental_options", &self.experimental_options)
            .finish()
    }
}

pub fn run() {
    let args = Arguments::from_args();
    output_capture::NO_CAPTURE
        .set(args.nocapture)
        .expect("should not be set already");

    let tests = TESTS
        .into_iter()
        .map(|test| {
            Trial::test(test.name.deref().to_string(), move || {
                output_capture::OUTPUT.with_borrow_mut(|output| output.clear());
                (test.function)()?;
                if args.show_output {
                    output_capture::OUTPUT.with_borrow(|output| {
                        for output in output {
                            if let Output::Stdout(output) = output {
                                print!("{output}");
                            }
                        }
                    });
                }
                // TODO: on error, show output
                Ok(())
            })
            .with_ignored_flag(test.ignored.0)
            .with_kind(test.experimental_options.to_string())
        })
        .collect();

    libtest_mimic::run(&args, tests).exit()
}
