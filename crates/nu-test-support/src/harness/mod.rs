use std::{
    borrow::Cow,
    collections::BTreeMap,
    env,
    fmt::{Debug, Write},
    ops::Deref,
    panic,
    sync::LazyLock,
};

use crate::{self as nu_test_support, harness::output_capture::Output};

use itertools::Itertools;
use libtest_with::{Arguments, Conclusion, Trial};
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
        let mut out = String::new();

        if !self.experimental_options.is_empty() {
            let opts = self
                .experimental_options
                .iter()
                .map(|(option, _value)| format!("{}={}", option.identifier(), _value))
                .join(", ");
            write!(out, "exp: {}", opts).unwrap();
        }

        if !self.experimental_options.is_empty() && !self.environment_variables.is_empty() {
            write!(out, "; ").unwrap();
        }

        if !self.environment_variables.is_empty() {
            let envs = self
                .environment_variables
                .iter()
                .map(|(key, value)| format!("{key}={value:?}"))
                .join(", ");
            write!(out, "env: {}", envs).unwrap();
        }

        out
    }

    fn experimental_options_sorted(&self) -> BTreeMap<&'static ExperimentalOption, bool> {
        self.experimental_options.iter().copied().collect()
    }

    fn environment_variables_sorted(&self) -> BTreeMap<&'static str, &'static str> {
        self.environment_variables.iter().copied().collect()
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

    let tests: BTreeMap<_, _> = TESTS
        .into_iter()
        .map(|test| {
            (
                (
                    test.experimental_options_sorted(),
                    test.environment_variables_sorted(),
                ),
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
                .with_kind(test.kind()),
            )
        })
        .into_group_map()
        .into_iter()
        .collect();

    let conclusion = tests
        .into_iter()
        .map(|(group, tests)| {
            let old_env_vars = group
                .1
                .iter()
                .map(|(key, _)| (key, env::var_os(key)))
                .collect_vec();
            group.1.iter().for_each(|(key, value)| unsafe {
                env::set_var(key, value);
            });
            group
                .0
                .into_iter()
                .for_each(|(option, value)| unsafe { option.set(value) });
            let conclusion = libtest_with::run(&args, tests);
            nu_experimental::ALL
                .iter()
                .for_each(|option| unsafe { option.unset() });
            old_env_vars.into_iter().for_each(|(key, value)| unsafe {
                match value {
                    Some(value) => env::set_var(key, value),
                    None => env::remove_var(key),
                }
            });
            conclusion
        })
        .fold(
            Conclusion {
                num_filtered_out: 0,
                num_passed: 0,
                num_failed: 0,
                num_ignored: 0,
                num_measured: 0,
            },
            |mut acc, c| {
                acc.num_filtered_out += c.num_filtered_out;
                acc.num_passed += c.num_passed;
                acc.num_failed += c.num_failed;
                acc.num_ignored += c.num_ignored;
                acc.num_measured += c.num_measured;
                acc
            },
        );

    conclusion.exit()
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
