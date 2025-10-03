use nu_experimental::{EXAMPLE, test_support::ExperimentalOptionsGuard};

#[test]
fn example_is_true() {
    let before = EXAMPLE.get();
    let mut guard = ExperimentalOptionsGuard::get();
    guard.set(&EXAMPLE, true);
    assert_eq!(EXAMPLE.get(), true);
    drop(guard);
    assert_eq!(EXAMPLE.get(), before);
}

#[test]
fn example_is_false() {
    let before = EXAMPLE.get();
    let mut guard = ExperimentalOptionsGuard::get();
    guard.set(&EXAMPLE, false);
    assert_eq!(EXAMPLE.get(), false);
    drop(guard);
    assert_eq!(EXAMPLE.get(), before);
}
