use super::*;

#[test]
fn a_folder_inside_the_checkout_holds_it_and_a_sibling_does_not() {
    let dir = tempfile::tempdir().expect("tempdir");
    let checkout = dir.path().join("card_1");
    std::fs::create_dir_all(checkout.join("src")).expect("mkdir");
    std::fs::create_dir_all(dir.path().join("card_10")).expect("mkdir");
    let real = |path: &Path| std::fs::canonicalize(path).expect("canonicalize");

    assert!(held(&[real(&checkout.join("src"))], &checkout));
    assert!(held(&[real(&checkout)], &checkout));
    assert!(!held(&[real(&dir.path().join("card_10"))], &checkout));
    assert!(!held(&[], &checkout));
}

#[cfg(target_os = "linux")]
#[test]
fn a_process_running_in_a_checkout_is_seen() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut sleeping = std::process::Command::new("sleep")
        .arg("30")
        .current_dir(dir.path())
        .spawn()
        .expect("sleep");

    let seen = held(&running_in(), dir.path());
    let _ = sleeping.kill();
    let _ = sleeping.wait();

    assert!(seen, "the process in the checkout went unseen");
    assert!(
        !held(&running_in(), dir.path()),
        "a process gone still holds it"
    );
}
