use std::{
    fmt::Debug,
    hash::{DefaultHasher, Hash, Hasher},
    ops::Deref,
    process::Termination,
};

use crate::{self as nu_test_support, harness::output_capture::Output};

#[doc(hidden)]
pub use linkme;
use nu_experimental::ExperimentalOption;

#[doc(hidden)]
pub use kitest::prelude::*;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct GroupKey(u64);

impl From<&TestMetaExtra> for GroupKey {
    fn from(extra: &TestMetaExtra) -> Self {
        let mut hasher = DefaultHasher::new();
        extra
            .experimental_options
            .iter()
            .map(|(opt, val)| (opt.identifier(), val))
            .for_each(|item| item.hash(&mut hasher));
        extra
            .environment_variables
            .iter()
            .for_each(|item| item.hash(&mut hasher));
        GroupKey(hasher.finish())
    }
}

fn grouper(meta: &TestMeta<TestMetaExtra>) -> GroupKey {
    GroupKey::from(&meta.extra)
}

pub fn main() -> impl Termination {
    kitest::harness(TESTS.deref()).with_grouper(grouper).run()
}
