use super::pending;

/// Claude Code asking with several tabs, as a real session drew it.
const ASKED: &str = "\
● Message from @orchestrator: answers to the four questions (ctrl+o to expand)
────────────────────────────────────────────────────────────────
← □ Slices  □ Repository  □ Handoff  □ Approval  ✔ Submit →

Large (8+ tasks, several goals). How do we split it? I propose 4 slices.

❯ 1. F1 spine + panel (Recommended)
     F1: model, internal endpoint, live panel with human handling.
  2. F1 with the worker
     F1 and F2 together: the first delivery already runs on its own.
  3. All at once
  4. Type something.
────────────────────────────────────────────────────────────────
  5. Chat about this
";

#[test]
fn a_question_with_its_choices_is_read_off_the_screen() {
    let found = pending(ASKED).expect("a question");
    assert_eq!(
        found.question,
        "Large (8+ tasks, several goals). How do we split it? I propose 4 slices."
    );
    let labels: Vec<&str> = found.options.iter().map(|one| one.label.as_str()).collect();
    // Below the rule is still a choice: the arrows reach it like the others.
    assert_eq!(
        labels,
        [
            "F1 spine + panel (Recommended)",
            "F1 with the worker",
            "All at once",
            "Type something.",
            "Chat about this"
        ]
    );
    assert_eq!(
        found.options[0].hint.as_deref(),
        Some("F1: model, internal endpoint, live panel with human handling.")
    );
    assert_eq!(found.options[2].hint, None);
    assert_eq!(found.cursor, 0);
}

#[test]
fn a_permission_prompt_is_one_too_with_the_cursor_where_it_is() {
    let screen = "\
 Bash command
   git push origin main
 Do you want to proceed?
   1. Yes
 ❯ 2. Yes, and don't ask again for git push commands
   3. No, and tell Claude what to do differently (esc)
";
    let found = pending(screen).expect("a question");
    assert_eq!(
        found.question,
        "Bash command git push origin main Do you want to proceed?"
    );
    assert_eq!(found.options.len(), 3);
    assert_eq!(found.cursor, 1);
}

#[test]
fn a_numbered_list_in_ordinary_output_is_not_a_question() {
    let screen = "Steps:\n1. build\n2. test\n3. ship\n$ ";
    assert_eq!(pending(screen), None);
}

#[test]
fn nothing_is_read_off_a_quiet_screen() {
    assert_eq!(pending(""), None);
    assert_eq!(pending("> \n"), None);
}

#[test]
fn a_permission_prompt_says_what_it_asks_about_across_its_paragraphs() {
    let screen = "\
────────────────────────────────
 Bash command

   git push origin main
   Push to remote

 Do you want to proceed?
 ❯ 1. Yes
   2. No, and tell Claude what to do differently (esc)
";
    let found = pending(screen).expect("a question");
    assert_eq!(
        found.question,
        "Bash command\ngit push origin main Push to remote\nDo you want to proceed?"
    );
}

#[test]
fn a_list_typed_into_the_input_box_is_the_persons_message_not_a_question() {
    let screen = "\
● Done.

────────────────────────────────
❯ 1. fix the parser
  2. add tests
────────────────────────────────
  ? for shortcuts
";
    assert_eq!(pending(screen), None);
}

#[test]
fn a_cursor_line_with_nothing_asked_above_it_is_not_a_question() {
    assert_eq!(pending("❯ 1. fix the parser\n  2. add tests\n"), None);
}

#[test]
fn choices_are_one_block_not_numbers_scattered_down_the_screen() {
    let screen = "Plan:\n1. build\n  blah\nSome text\nmore\n❯ 2. whatever\n";
    assert_eq!(pending(screen), None);
}

#[test]
fn a_lone_choice_is_not_a_question() {
    assert_eq!(pending("Which one?\n❯ 1. fix the build\n"), None);
}

#[test]
fn a_run_that_does_not_start_at_one_is_not_read_and_a_gap_ends_one() {
    assert_eq!(pending("Which?\n❯ 3. A\n  4. B\n"), None);
    let found = pending("Which?\n❯ 1. A\n  2. B\n  4. C\n").expect("a question");
    assert_eq!(found.options.len(), 2);
}

#[test]
fn the_question_on_screen_now_is_the_lowest_one() {
    let screen = "\
Old one?
❯ 1. Before
  2. Earlier
● Answered.

New one?
  1. Now
❯ 2. Later
";
    let found = pending(screen).expect("a question");
    assert_eq!(found.question, "New one?");
    assert_eq!(found.cursor, 1);
    assert_eq!(found.options[0].label, "Now");
}

#[test]
fn a_footer_level_with_a_choice_is_not_its_description() {
    let screen = "Proceed?\n ❯ 1. Yes\n   2. No\n   Esc to cancel\n";
    let found = pending(screen).expect("a question");
    assert_eq!(found.options[1].hint, None);
}
