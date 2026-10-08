use super::ran;

#[cfg(unix)]
#[test]
fn a_command_past_its_timeout_is_stopped_and_fails_the_preparation() {
    let dir = tempfile::tempdir().expect("tempdir");
    let began = std::time::Instant::now();
    let mut said = Vec::new();
    let code = ran(
        "echo started; sleep 30",
        dir.path(),
        &Default::default(),
        Some(1),
        &dir.path().join("log"),
        &mut |line: &str| said.push(line.to_owned()),
    )
    .expect("ran");
    assert!(began.elapsed() < std::time::Duration::from_secs(10));
    assert_eq!(code, Some(124));
    assert!(said.iter().any(|line| line.contains("started")));
    assert!(said.iter().any(|line| line == "stopped after 1 s"));
}
