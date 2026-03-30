use nu_experimental::EXAMPLE;
use nu_test_support::prelude::*;
use rstest::rstest;
use std::collections::HashMap;

#[test]
fn lists_expected_fields_for_example() -> Result {
    let code = r#"
        debug experimental-options
        | where identifier == example
        | get 0
    "#;

    let outcome: HashMap<String, Value> = test().run(code)?;
    assert_eq!(outcome["identifier"].as_str()?, "example");
    let _ = outcome["enabled"].as_bool()?;
    let _ = outcome["status"].as_str()?;
    let _ = outcome["description"].as_str()?;
    let _ = outcome["since"].as_str()?;
    let issue = outcome["issue"].as_str()?;
    assert_contains("https://github.com/nushell/nushell/issues/", issue);
    Ok(())
}

#[rstest]
#[nu_test_support::test]
#[exp(EXAMPLE = true)]
#[case(true)]
#[nu_test_support::test]
#[exp(EXAMPLE = false)]
#[case(false)]
fn respects_experimental_option_setting(#[case] expected: bool) -> Result {
    let code = "debug experimental-options | where identifier == example | get enabled.0";
    test().run(code).expect_value_eq(expected)
}

#[test]
fn experimental_options_is_const() -> Result {
    let code = r#"
        const options = debug experimental-options
        $options
        | where identifier == example
        | get enabled.0
    "#;
    test().run(code).expect_value_eq(false)
}
