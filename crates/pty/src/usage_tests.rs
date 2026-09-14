use super::*;

/// A real line, taken from this machine.
const STAT: &str = "3537747 (bash) S 3537740 3537747 3537747 34816 3537747 4194304 1234 0 0 0 42 17 0 0 20 0 1 0 123456 12345678 987 18446744073709551615 1 2 3";

#[test]
fn the_parent_is_the_fourth_field() {
    assert_eq!(parse_parent(STAT), Some(3537740));
}

#[test]
fn a_program_may_be_called_something_that_looks_like_the_rest_of_the_line() {
    // The second field is the executable name in brackets, and a program is
    // allowed to be called `foo) 1 2 3 (bar`. Splitting the whole line reads
    // the name as the fields.
    let hostile = "42 (foo) 1 2 3 (bar) S 99 42 42 0 -1 0 0 0 0 0 7 3 0 0 20 0 1 0 1 1 1";
    assert_eq!(parse_parent(hostile), Some(99));
}

#[test]
fn cpu_time_is_user_plus_system() {
    assert_eq!(parse_ticks(STAT), Some(42 + 17));
}

#[test]
fn a_line_that_is_not_a_stat_line_answers_nothing() {
    assert_eq!(parse_parent("nonsense"), None);
    assert_eq!(parse_ticks("nonsense"), None);
}

#[test]
fn a_memory_field_is_read_in_kibibytes() {
    let rollup = "Rss:              123456 kB\nPss:               65432 kB\nShared_Clean: 1 kB";
    assert_eq!(parse_field(rollup, "Pss:"), Some(65432));
    assert_eq!(parse_field(rollup, "Rss:"), Some(123456));
}

#[test]
fn a_field_that_is_not_there_answers_nothing() {
    assert_eq!(parse_field("Rss: 1 kB", "Pss:"), None);
}

#[test]
fn a_prefix_of_another_field_is_not_that_field() {
    // `Rss:` must not match `RssAnon:`, which is a different number.
    let rollup = "RssAnon:            999 kB\nRss:              111 kB";
    assert_eq!(parse_field(rollup, "Rss:"), Some(111));
}

#[cfg(target_os = "linux")]
#[test]
fn this_process_is_in_its_own_tree() {
    let mine = std::process::id();
    assert!(tree(&[mine]).contains(&mine));
}

#[cfg(target_os = "linux")]
#[test]
fn this_process_costs_something_and_says_how_it_was_counted() {
    let mine = std::process::id();
    let found = costs(&[mine]);
    assert_eq!(found.len(), 1);
    assert!(found[0].memory > 0, "a running process uses memory");
    assert!(
        found[0].shared,
        "our own process can be read proportionally, so it should be"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn a_process_that_is_gone_is_left_out_rather_than_guessed_at() {
    // Between listing and asking, a pane closes. That is ordinary.
    assert_eq!(costs(&[u32::MAX]), Vec::new());
}
