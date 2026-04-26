use std::{env, num::NonZeroUsize, process::ExitCode};

use kitest::runner::DefaultRunner;
use nu_test::group::{ModuleRunner, TestModules};

// TODO: remove this file, we expose main function via `nu` crate then

fn main() {
    let engine_state = nu_test::engine_state();
    let discoveries =
        nu_test::discover::discover_recursively(&engine_state, "crates/nu-std/tests").unwrap();
    let cwd = env::current_dir().unwrap();
    let test_iter = discoveries
        .into_iter()
        .map(|discovery| nu_test::test::build_tests(discovery, &cwd));
    let mut all_tests = Vec::new();
    let mut test_modules = TestModules::new();
    for (tests, test_module) in test_iter.into_iter().filter(|(tests, _)| tests.len() > 0) {
        all_tests.extend(tests);
        test_modules.insert(all_tests.last().expect("not empty").extra.module_name.clone(), test_module);
    }

    let _ = kitest::harness(&all_tests)
        .with_grouper(test_modules)
        .with_runner(DefaultRunner::default().with_thread_count(NonZeroUsize::MIN))
        .with_group_runner(ModuleRunner)
        .run();
}
