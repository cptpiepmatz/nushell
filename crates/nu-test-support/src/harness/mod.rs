use std::{
    collections::BTreeMap, env, fmt::{Debug, Write}, ops::Deref, panic, process::Termination, sync::LazyLock
};

use crate::{self as nu_test_support, harness::output_capture::Output};

use itertools::Itertools;
use libtest_with::{Arguments, Conclusion, Trial};
#[doc(hidden)]
pub use linkme;
use nu_experimental::ExperimentalOption;

#[doc(hidden)]
pub use kitest;

pub mod output_capture;
pub mod macros {
    pub use linkme::distributed_slice as collect_test;
    pub use nu_test_support_macros::test;
}

/// All collected tests.
#[linkme::distributed_slice]
#[linkme(crate = nu_test_support::harness::linkme)]
pub static TESTS: [kitest::test::Test<TestMetaExtra>];

pub struct TestMetaExtra {
    pub experimental_options: &'static [(&'static ExperimentalOption, bool)],
    pub environment_variables: &'static [(&'static str, &'static str)],
}

pub fn main() {

}

