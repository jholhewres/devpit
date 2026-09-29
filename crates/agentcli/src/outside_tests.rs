use super::*;

#[test]
fn a_root_becomes_the_folder_the_cli_uses() {
    // Measured: the dot in `jhol.dev` is replaced as well as the slashes.
    assert_eq!(
        folder_name(Path::new("/home/jhol/Workspace/private/jhol.dev")),
        "-home-jhol-Workspace-private-jhol-dev"
    );
}

#[test]
fn an_underscore_becomes_a_dash_like_every_other_non_alphanumeric() {
    // Measured: the CLI keeps `/home/a/whmcs_br/led-billing` under
    // `-home-a-whmcs-br-led-billing`, and a folder keeping the `_` is empty.
    assert_eq!(
        folder_name(Path::new("/home/a/whmcs_br/led billing@2")),
        "-home-a-whmcs-br-led-billing-2"
    );
}

#[test]
fn the_last_title_wins() {
    let transcript = concat!(
        "{\"type\":\"user\",\"message\":{\"content\":\"hi\"}}\n",
        "{\"type\":\"ai-title\",\"aiTitle\":\"First guess\",\"sessionId\":\"s\"}\n",
        "{\"type\":\"assistant\",\"message\":{\"content\":[]}}\n",
        "{\"type\":\"ai-title\",\"aiTitle\":\"Fix the parser\",\"sessionId\":\"s\"}\n",
    );
    assert_eq!(
        last_title(transcript.as_bytes()).as_deref(),
        Some("Fix the parser")
    );
}

#[test]
fn a_transcript_with_no_title_has_none() {
    assert_eq!(last_title("{\"type\":\"user\"}\n".as_bytes()), None);
}

/// A line past the ceiling is skipped, and the title after it still read: a
/// tool output of megabytes must neither be held nor hide what follows.
#[test]
fn an_oversized_line_is_skipped_not_held() {
    let huge = format!(
        "{{\"type\":\"ai-title\",\"aiTitle\":\"{}\"}}\n",
        "x".repeat(MOST_LINE_BYTES + 10)
    );
    let transcript = format!("{huge}{{\"type\":\"ai-title\",\"aiTitle\":\"Kept\"}}\n");
    assert_eq!(last_title(transcript.as_bytes()).as_deref(), Some("Kept"));
}

#[test]
fn a_last_line_without_a_newline_is_still_read() {
    assert_eq!(
        last_title("{\"type\":\"ai-title\",\"aiTitle\":\"End\"}".as_bytes()).as_deref(),
        Some("End")
    );
}

#[test]
fn sessions_devpit_already_holds_are_left_out() {
    let install = tempfile::tempdir().expect("tempdir");
    let root = Path::new("/work/demo.app");
    let dir = install.path().join("projects").join(folder_name(root));
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(
        dir.join("aaa.jsonl"),
        "{\"type\":\"ai-title\",\"aiTitle\":\"Terminal work\"}\n",
    )
    .expect("write");
    std::fs::write(dir.join("bbb.jsonl"), "{\"type\":\"user\"}\n").expect("write");
    std::fs::write(dir.join("notes.txt"), "not a session").expect("write");

    let known: HashSet<String> = ["bbb".to_owned()].into();
    let found = sessions(&[install.path().to_path_buf()], root, &known);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].session_id, "aaa");
    assert_eq!(found[0].title.as_deref(), Some("Terminal work"));
    assert_eq!(found[0].installation, install.path());
}

#[test]
fn a_project_the_cli_never_ran_in_has_no_sessions() {
    let install = tempfile::tempdir().expect("tempdir");
    assert!(sessions(
        &[install.path().to_path_buf()],
        Path::new("/nowhere"),
        &HashSet::new()
    )
    .is_empty());
}
