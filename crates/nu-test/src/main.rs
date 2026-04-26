use std::{env, num::NonZeroUsize, process::ExitCode};

use kitest::runner::DefaultRunner;
use nu_test::group::{ModuleRunner, TestModules};

// TODO: remove this file, we expose main function via `nu` crate then

fn main() -> ExitCode {
    let engine_state = nu_test::engine_state();
    let discovery =
        nu_test::discover::discover(engine_state, "crates/nu-test/tests/example.nu").unwrap();
    let cwd = env::current_dir().unwrap();
    let (tests, test_module) = nu_test::test::build_tests(discovery, cwd);
    let tests: Vec<_> = tests.collect();
    let mut test_modules = TestModules::new();
    test_modules.insert(tests[0].extra.module_name.clone(), test_module);
    kitest::harness(&tests)
        .with_grouper(test_modules)
        .with_runner(DefaultRunner::default().with_thread_count(NonZeroUsize::MIN))
        .with_group_runner(ModuleRunner)
        .run()
        .exit_code()
}
