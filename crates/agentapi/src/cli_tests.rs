use super::*;

fn words(line: &str) -> Vec<String> {
    line.split_whitespace().map(ToOwned::to_owned).collect()
}

#[test]
fn nothing_or_help_is_the_guide() {
    assert_eq!(parsed(&[]), Ok(Parsed::Guide));
    assert_eq!(parsed(&words("--help")), Ok(Parsed::Guide));
}

#[test]
fn a_read_names_its_method() {
    assert_eq!(
        parsed(&words("board")),
        Ok(Parsed::Ask("board".into(), json!({})))
    );
    assert_eq!(
        parsed(&words("card crd_1")),
        Ok(Parsed::Ask("card".into(), json!({ "cardId": "crd_1" })))
    );
}

#[test]
fn a_comment_is_every_word_after_the_card() {
    assert_eq!(
        parsed(&words("comment crd_1 tests pass on main")),
        Ok(Parsed::Ask(
            "comment".into(),
            json!({ "cardId": "crd_1", "body": "tests pass on main" })
        ))
    );
    assert!(parsed(&words("comment crd_1")).is_err());
}

#[test]
fn options_travel_under_the_names_the_app_reads() {
    assert_eq!(
        parsed(&words("create Fix login --body steps --column col_2")),
        Ok(Parsed::Ask(
            "create".into(),
            json!({ "title": "Fix login", "body": "steps", "columnId": "col_2" })
        ))
    );
    assert_eq!(
        parsed(&words("update crd_1 --title New")),
        Ok(Parsed::Ask(
            "update".into(),
            json!({ "cardId": "crd_1", "title": "New" })
        ))
    );
}

#[test]
fn an_unknown_command_is_refused_by_name() {
    assert_eq!(
        parsed(&words("deploy")),
        Err("`deploy` is not a devpit agent command".to_owned())
    );
}
