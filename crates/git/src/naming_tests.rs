use crate::fixture;
use crate::naming::*;

#[test]
fn a_new_cards_branch_is_its_type_number_and_a_short_slug() {
    assert_eq!(
        branch_for(
            "",
            "Chat 3/8: base visual — type, radius, spacing",
            Some(322),
            "card_01M7AR67"
        ),
        "feat/322-chat-3-8-base-visual-type"
    );
}

#[test]
fn a_bug_is_a_fix_and_its_word_leaves_the_slug() {
    assert_eq!(
        branch_for("", "Fix flaky macOS tests", Some(351), "card_01M7AR67"),
        "fix/351-flaky-macos-tests"
    );
    assert_eq!(
        branch_for("", "Bug: the login loops", Some(7), "card_01M7AR67"),
        "fix/7-the-login-loops"
    );
}

#[test]
fn a_title_that_names_a_type_gets_it_and_any_other_is_a_feature() {
    let kind = |title| branch_for("{type}", title, Some(1), "card_01M7AR67");
    assert_eq!(kind("Docs: the setup page"), "docs");
    assert_eq!(kind("Test the sweep"), "test");
    assert_eq!(kind("chore(deps): bump"), "chore");
    assert_eq!(kind("Refactor the store"), "refactor");
    assert_eq!(kind("Testing harness"), "feat");
    assert_eq!(kind("Add a branch pattern"), "feat");
}

#[test]
fn a_long_title_gives_a_slug_of_whole_words_and_no_id() {
    let branch = branch_for(
        "",
        "Branch names: conventional type prefix and card number instead of a long slug",
        Some(352),
        "card_01M7BKWVMXV3XYAH2JBDT58VWJ",
    );
    assert_eq!(branch, "feat/352-branch-names-conventional-type");
    let slug = branch.trim_start_matches("feat/352-");
    assert!(slug.len() <= 30, "{slug} is {} long", slug.len());
    assert!(!branch.contains("bdt58vwj"), "{branch} carries the id");
}

#[test]
fn an_accented_title_keeps_its_letters() {
    assert_eq!(
        branch_for(
            "",
            "Telemetria e logs de produção",
            Some(9),
            "card_01M38MRZ"
        ),
        "feat/9-telemetria-e-logs-de-producao"
    );
}

#[test]
fn a_title_with_nothing_nameable_in_it_still_gets_a_branch() {
    assert_eq!(
        branch_for("", "!!! ???", Some(5), "card_abcdefgh"),
        "feat/5-card"
    );
}

#[test]
fn a_card_without_a_number_is_named_by_its_id() {
    assert_eq!(
        branch_for("", "Fix it", None, "card_01HZXY9K"),
        "fix/01hzxy9k-it"
    );
}

#[test]
fn the_older_style_can_still_be_chosen() {
    assert_eq!(
        branch_for(
            "devpit/{slug}-{id}",
            "Fix the login redirect",
            Some(4),
            "card_01HZXY9K"
        ),
        "devpit/fix-the-login-redirect-01hzxy9k"
    );
}

#[test]
fn a_name_already_taken_gets_a_number() {
    let dir = tempfile::tempdir().expect("tempdir");
    fixture::repo(dir.path());
    std::fs::write(dir.path().join("a.txt"), "one\n").expect("write");
    fixture::commit(dir.path(), "first");

    assert_eq!(unused_branch(dir.path(), "feat/1-x"), "feat/1-x");
    crate::run(dir.path(), &["branch", "feat/1-x"]).expect("branch");
    assert_eq!(unused_branch(dir.path(), "feat/1-x"), "feat/1-x-2");
    crate::run(dir.path(), &["branch", "feat/1-x-2"]).expect("branch");
    assert_eq!(unused_branch(dir.path(), "feat/1-x"), "feat/1-x-3");
}

#[test]
fn a_pattern_is_refused_when_it_cannot_name_a_cards_branch() {
    assert!(pattern_allowed("").is_ok());
    assert!(pattern_allowed(BRANCH_PATTERN).is_ok());
    assert!(pattern_allowed("devpit/{slug}-{id}").is_ok());
    assert!(
        pattern_allowed("work").is_err(),
        "the same name for every card"
    );
    assert!(pattern_allowed("{type}/{nope}-{slug}").is_err());
    assert!(pattern_allowed("{type}/{number} {slug}").is_err());
    assert!(pattern_allowed("{type}..{slug}").is_err());
}
