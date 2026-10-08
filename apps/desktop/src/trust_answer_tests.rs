use super::{plain_trust, Cursor};

/// Claude Code 2.1.294's question: no numbers, "No" first and under the cursor.
const ASKED: &str = "\
 Accessing workspace:

 /home/a/new-project

 Quick safety check: Is this a project you created or one you trust?

 ❯ No, exit
   Yes, I trust this folder

 Enter to confirm · Esc to cancel
";

#[test]
fn the_plain_question_is_seen_with_the_cursor_on_no() {
    assert_eq!(plain_trust(ASKED), Some(Cursor::Above));
    let moved = ASKED
        .replace("❯ No, exit", "  No, exit")
        .replace("  Yes, I trust", "❯ Yes, I trust");
    assert_eq!(plain_trust(&moved), Some(Cursor::OnYes));
}

#[test]
fn a_folder_that_brings_its_own_permissions_is_left_for_the_person() {
    let brings = ASKED.replace("No, exit", "No, continue without these permissions")
        + " This folder pre-approves 3 tool permissions\n";
    assert_eq!(plain_trust(&brings), None);
}

#[test]
fn an_older_question_with_yes_first_is_seen_too() {
    let older = " Do you trust the files in this folder?\n\n ❯ 1. Yes, proceed\n   2. No, exit\n";
    assert_eq!(plain_trust(older), Some(Cursor::OnYes));
}

#[test]
fn a_session_that_is_up_is_not_a_trust_question() {
    // "Yes, I trust this folder" quoted in a conversation, with no choice drawn.
    let talking = " > why does it say Yes, I trust this folder?\n\n ❯ \n";
    assert_eq!(plain_trust(talking), None);
}
