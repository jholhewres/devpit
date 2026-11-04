use super::*;

#[test]
fn only_pictures_are_taken() {
    assert_eq!(extension_for("image/png"), Some("png"));
    assert_eq!(extension_for("image/jpeg"), Some("jpg"));
    assert_eq!(extension_for("text/html"), None);
    assert_eq!(extension_for("application/octet-stream"), None);
    assert_eq!(extension_for("audio/webm;codecs=opus"), Some("webm"));
    assert_eq!(extension_for("audio/mp4"), Some("m4a"));
    assert_eq!(extension_for("audio/x-anything"), None);
}

#[test]
fn a_small_paste_decodes() {
    assert_eq!(decoded("aGk=").expect("decoded"), b"hi");
}

/// Refused on its length, before a byte is decoded: the ceiling is what keeps
/// a paste from being held whole in memory.
#[test]
fn a_paste_past_the_ceiling_is_refused_before_decoding() {
    let huge = "A".repeat(MOST_BYTES / 3 * 4 + 8);
    let refused = decoded(&huge).expect_err("refused");
    assert_eq!(refused.code, ErrorCode::Invalid);
}

#[test]
fn text_that_is_not_base64_is_refused() {
    assert!(decoded("not base64 at all!").is_err());
}

#[test]
fn a_project_id_that_is_a_path_is_refused() {
    let refused = chat_paste_now("../x".to_owned(), "image/png".to_owned(), "aGk=".to_owned())
        .expect_err("refused");
    assert_eq!(refused.code, ErrorCode::Forbidden);
}

#[test]
fn a_picture_is_read_inside_its_roots_and_nowhere_else() {
    let project = tempfile::tempdir().expect("project");
    let pasted = tempfile::tempdir().expect("pasted");
    let outside = tempfile::tempdir().expect("outside");
    std::fs::write(project.path().join("shot.png"), b"png").expect("write");
    std::fs::write(pasted.path().join("pasted-1.png"), b"png").expect("write");
    std::fs::write(project.path().join("notes.txt"), b"text").expect("write");
    std::fs::write(outside.path().join("secret.png"), b"png").expect("write");
    let roots = [project.path(), pasted.path()];

    assert_eq!(
        picture_in(&roots, "shot.png").expect("relative"),
        "data:image/png;base64,cG5n"
    );
    let kept = pasted.path().join("pasted-1.png");
    assert!(picture_in(&roots, &kept.display().to_string()).is_ok());
    let elsewhere = outside.path().join("secret.png");
    assert_eq!(
        picture_in(&roots, &elsewhere.display().to_string())
            .expect_err("outside")
            .code,
        ErrorCode::Forbidden
    );
    assert!(picture_in(&roots, "../secret.png").is_err());
    assert_eq!(
        picture_in(&roots, "notes.txt").expect_err("text").code,
        ErrorCode::Invalid
    );
}
