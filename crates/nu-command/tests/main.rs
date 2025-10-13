mod commands;
mod format_conversions;
mod sort_utils;
mod string;

#[macro_use]
extern crate nu_test_support;
use nu_test_support::harness::main;

const fn dynamic_somehow() -> &'static str {
    "yeah!"
}

#[test]
#[experimental_options(nu_experimental::EXAMPLE)]
#[env(SOME_ENV = "lol", DYNAMIC = dynamic_somehow())]
fn experimental_option_test() {}
