use super::*;

#[test]
fn only_pictures_are_taken() {
    assert_eq!(extension_for("image/png"), Some("png"));
    assert_eq!(extension_for("image/jpeg"), Some("jpg"));
    assert_eq!(extension_for("text/html"), None);
    assert_eq!(extension_for("application/octet-stream"), None);
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
fn pastes_live_in_the_workspace_not_the_project() {
    let dir = folder(Path::new("/home/me/.devpit"), "prj_1");
    assert_eq!(dir, Path::new("/home/me/.devpit/projects/prj_1/pasted"));
}

#[test]
fn a_project_id_that_is_a_path_is_refused() {
    let refused = chat_paste("../x".to_owned(), "image/png".to_owned(), "aGk=".to_owned())
        .expect_err("refused");
    assert_eq!(refused.code, ErrorCode::Forbidden);
}
