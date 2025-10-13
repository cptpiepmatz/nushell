use std::{
    borrow::Cow,
    fmt::{Debug, Write},
    ops::Deref,
    panic,
    sync::LazyLock,
};

use crate::{self as nu_test_support, harness::output_capture::Output};

use libtest_with::{Arguments, Trial};
#[doc(hidden)]
pub use linkme;
use nu_experimental::ExperimentalOption;

pub mod output_capture;
pub mod macros {
    pub use linkme::distributed_slice as collect_test;
    pub use nu_test_support_macros::test;
}

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
    pub experimental_options: &'static [(&'static ExperimentalOption, bool)],
    /// Configured environment variables to run test with.
    pub environment_variables: &'static [(&'static str, &'static str)],
}

impl TestMetadata {
    fn kind(&self) -> String {
        // calling fmt::Write on a String is infallible
        let mut out = String::new();

        if !self.experimental_options.is_empty() {
            write!(out, "exp: ").unwrap();
            let mut first = true;
            for (option, value) in self.experimental_options.iter() {
                if !first {
                    write!(out, ", ").unwrap()
                };
                first = false;
                write!(
                    out,
                    "{identifier}={value}",
                    identifier = option.identifier()
                )
                .unwrap();
            }
        }

        if !self.experimental_options.is_empty() && !self.environment_variables.is_empty() {
            write!(out, "; ").unwrap();
        }

        if !self.environment_variables.is_empty() {
            write!(out, "env: ").unwrap();
            let mut first = true;
            for (key, value) in self.environment_variables.iter() {
                if !first {
                    write!(out, ", ").unwrap()
                };
                first = false;
                write!(out, "{key}={value:?}").unwrap();
            }
        }

        out
    }
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

pub fn main() {
    let args = Arguments::from_args();
    output_capture::NO_CAPTURE
        .set(args.nocapture)
        .expect("should not be set already");

    let tests = TESTS
        .into_iter()
        .map(|test| {
            Trial::test(test.name.deref().to_string(), move || {
                output_capture::OUTPUT.with_borrow_mut(|output| output.clear());
                let test_run = panic::catch_unwind(test.function);
                match (test.should_panic.0, test_run) {
                    (true, Err(_)) => (),
                    (false, Ok(_)) => (),
                    (_, Err(err)) => todo!("handle unexpected panic: {}", panic_message(err)),
                    (true, Ok(_)) => todo!("handle expected panic"),
                }
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
            .with_ignored_flag(test.ignored.0, test.ignored.1.map(String::from))
            .with_kind(test.kind())
        })
        .collect();

    libtest_with::run(&args, tests).exit()
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        format!("<non-string panic payload: {:?}>", payload.type_id())
    }
}
