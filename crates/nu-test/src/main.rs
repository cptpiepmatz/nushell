use std::process::ExitCode;

// TODO: remove this file, we expose main function via `nu` crate then

fn main() -> ExitCode {
    let engine_state = nu_test::engine_state();
    let discovery =
        nu_test::discover::discover(engine_state, "crates/nu-test/tests/example.nu").unwrap();
    let tests: Vec<_> = nu_test::test::build_tests(discovery).collect();
    kitest::harness(&tests).run().exit_code()
}
