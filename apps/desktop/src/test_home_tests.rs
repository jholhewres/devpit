#[test]
fn the_tests_keep_devpit_in_a_home_of_their_own() {
    let root = devpit_core::Store::root().unwrap();
    assert!(
        root.starts_with(super::home()),
        "devpit's root in a test is {}",
        root.display()
    );
}
