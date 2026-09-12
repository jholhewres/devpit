use super::*;
use crate::fixture;

#[test]
fn args_for_maps_each_flag_combination() {
    for case in [true, false] {
        for word in [true, false] {
            for regex in [true, false] {
                let argv = args_for(SearchFlags { case, word, regex });
                assert!(
                    argv.contains(&"-z"),
                    "case={case} word={word} regex={regex}"
                );
                assert_eq!(
                    argv.contains(&"-i"),
                    !case,
                    "case={case} word={word} regex={regex}"
                );
                assert_eq!(
                    argv.contains(&"-w"),
                    word,
                    "case={case} word={word} regex={regex}"
                );
                assert_eq!(
                    argv.contains(&"-E"),
                    regex,
                    "case={case} word={word} regex={regex}"
                );
                assert_eq!(
                    argv.contains(&"-F"),
                    !regex,
                    "case={case} word={word} regex={regex}"
                );
            }
        }
    }
}

#[test]
fn a_pattern_past_the_ceiling_is_refused() {
    let huge = "a".repeat(MAX_PATTERN_BYTES + 1);
    assert!(within_pattern_ceiling(&huge).is_err());
}

#[test]
fn a_pattern_at_the_ceiling_is_allowed() {
    let fits = "a".repeat(MAX_PATTERN_BYTES);
    assert!(within_pattern_ceiling(&fits).is_ok());
}

#[test]
fn a_pattern_past_the_ceiling_is_refused_by_grep_too() {
    let dir = tempfile::tempdir().expect("tempdir");
    fixture::repo(dir.path());
    let huge = "a".repeat(MAX_PATTERN_BYTES + 1);
    let err = grep(dir.path(), &huge, SearchFlags::default(), 10).unwrap_err();
    assert!(matches!(err, GitError::Refused(_)));
}

#[test]
fn a_semicolon_in_the_pattern_is_searched_literally() {
    // If this ever reached a shell, `;` would end the command rather than be
    // part of the text it is asked to find.
    let dir = tempfile::tempdir().expect("tempdir");
    fixture::repo(dir.path());
    std::fs::write(dir.path().join("a.txt"), "rm -rf /; echo done\n").expect("write");
    fixture::commit(dir.path(), "add a.txt");

    let outcome = grep(dir.path(), "rf /;", SearchFlags::default(), 10).expect("grep");
    assert_eq!(outcome.hits.len(), 1);
    assert_eq!(outcome.hits[0].path, "a.txt");
}

#[test]
fn a_literal_pattern_with_regex_syntax_is_matched_as_text_under_dash_f() {
    let dir = tempfile::tempdir().expect("tempdir");
    fixture::repo(dir.path());
    std::fs::write(dir.path().join("a.txt"), "if (ready) run();\n").expect("write");
    fixture::commit(dir.path(), "add a.txt");

    let flags = SearchFlags {
        case: true,
        word: false,
        regex: false,
    };
    let outcome = grep(dir.path(), "(ready)", flags, 10).expect("grep");
    assert_eq!(outcome.hits.len(), 1);
}

#[test]
fn nothing_matching_answers_empty_rather_than_failing() {
    let dir = tempfile::tempdir().expect("tempdir");
    fixture::repo(dir.path());
    std::fs::write(dir.path().join("a.txt"), "one\n").expect("write");
    fixture::commit(dir.path(), "add a.txt");

    let outcome = grep(
        dir.path(),
        "nowhere to be found",
        SearchFlags::default(),
        10,
    )
    .expect("grep");
    assert!(outcome.hits.is_empty());
    assert_eq!(outcome.matched, 0);
}

#[test]
fn hits_past_the_ceiling_are_counted_but_not_collected() {
    let dir = tempfile::tempdir().expect("tempdir");
    fixture::repo(dir.path());
    std::fs::write(dir.path().join("a.txt"), "needle\nneedle\nneedle\n").expect("write");
    fixture::commit(dir.path(), "add a.txt");

    let outcome = grep(dir.path(), "needle", SearchFlags::default(), 2).expect("grep");
    assert_eq!(outcome.hits.len(), 2);
    assert_eq!(outcome.matched, 3);
}

#[test]
fn a_line_past_the_ceiling_is_shortened_and_marked() {
    let dir = tempfile::tempdir().expect("tempdir");
    fixture::repo(dir.path());
    let long_line = format!("needle {}\n", "x".repeat(MAX_LINE_CHARS + 50));
    std::fs::write(dir.path().join("a.txt"), &long_line).expect("write");
    fixture::commit(dir.path(), "add a.txt");

    let outcome = grep(dir.path(), "needle", SearchFlags::default(), 10).expect("grep");
    assert_eq!(outcome.hits.len(), 1);
    let text = &outcome.hits[0].text;
    assert!(text.chars().count() <= MAX_LINE_CHARS + 1);
    assert!(text.ends_with('…'));
}

#[test]
fn word_mode_does_not_match_inside_a_longer_word() {
    let dir = tempfile::tempdir().expect("tempdir");
    fixture::repo(dir.path());
    std::fs::write(dir.path().join("a.txt"), "needled\n").expect("write");
    fixture::commit(dir.path(), "add a.txt");

    let flags = SearchFlags {
        case: true,
        word: true,
        regex: false,
    };
    let outcome = grep(dir.path(), "needle", flags, 10).expect("grep");
    assert!(outcome.hits.is_empty());
}

#[test]
fn a_colon_in_the_path_does_not_swallow_the_line_number() {
    // Under the old `path:line:text` split on `:`, a colon in the filename
    // shifted every field after it — the line number failed to parse and the
    // hit was silently dropped. `-z` cannot have this problem: `\0` cannot
    // appear in a filename.
    let dir = tempfile::tempdir().expect("tempdir");
    fixture::repo(dir.path());
    std::fs::write(dir.path().join("weird:name.txt"), "needle\n").expect("write");
    fixture::commit(dir.path(), "add weird:name.txt");

    let outcome = grep(dir.path(), "needle", SearchFlags::default(), 10).expect("grep");
    assert_eq!(outcome.hits.len(), 1);
    assert_eq!(outcome.hits[0].path, "weird:name.txt");
    assert_eq!(outcome.hits[0].line, 1);
}

#[test]
fn a_single_file_cannot_fill_the_ceiling_by_itself() {
    // `-m` is enforced by git, not by `parse` — proven by asking for far more
    // hits than `-m` allows and getting back exactly `-m`'s worth, even
    // though `ceiling` here would happily hold them all.
    let dir = tempfile::tempdir().expect("tempdir");
    fixture::repo(dir.path());
    let many = "needle\n".repeat(MAX_MATCHES_PER_FILE + 20);
    std::fs::write(dir.path().join("a.txt"), many).expect("write");
    fixture::commit(dir.path(), "add a.txt");

    let outcome = grep(dir.path(), "needle", SearchFlags::default(), 10_000).expect("grep");
    assert_eq!(outcome.matched, MAX_MATCHES_PER_FILE);
}
