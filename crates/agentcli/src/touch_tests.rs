use serde_json::json;

use super::*;

#[test]
fn a_file_step_reads_as_the_file_name() {
    let input = json!({ "file_path": "/w/app/src/invoice.ts", "old_string": "a" });
    assert_eq!(target_of(&input).as_deref(), Some("invoice.ts"));
}

#[test]
fn a_command_step_reads_as_its_first_line() {
    let input = json!({ "command": "npm test\n&& echo done" });
    assert_eq!(target_of(&input).as_deref(), Some("npm test"));
}

#[test]
fn a_search_reads_as_its_pattern_and_a_fetch_as_its_address() {
    assert_eq!(
        target_of(&json!({ "pattern": "fn main" })).as_deref(),
        Some("fn main")
    );
    assert_eq!(
        target_of(&json!({ "url": "https://docs.rs/serde" })).as_deref(),
        Some("docs.rs/serde")
    );
}

/// The closed list: a field nobody recorded is not read.
#[test]
fn an_input_with_none_of_the_known_fields_has_no_target() {
    assert_eq!(target_of(&json!({ "whatever": "x" })), None);
    assert_eq!(target_of(&json!({ "command": "   " })), None);
}

/// Cut on a character, never inside one: a target is drawn, and half a
/// character is a replacement glyph on screen.
#[test]
fn a_long_target_is_cut_on_a_character() {
    let long = "é".repeat(TARGET + 5);
    let target = target_of(&json!({ "command": long })).expect("a target");
    assert_eq!(target.chars().count(), TARGET + 1);
    assert!(target.ends_with('…'));
}

#[test]
fn an_edit_keeps_both_sides_for_the_preview() {
    let input = json!({ "file_path": "/w/a.rs", "old_string": "x = 1", "new_string": "x = 2" });
    assert_eq!(
        touch_of("Edit", &input),
        Some(Touch::Edit {
            path: "/w/a.rs".to_owned(),
            before: "x = 1".to_owned(),
            after: "x = 2".to_owned(),
        })
    );
}

#[test]
fn a_read_keeps_where_it_starts() {
    let input = json!({ "file_path": "/w/a.rs", "offset": 10, "limit": 20 });
    assert_eq!(
        touch_of("Read", &input),
        Some(Touch::Read {
            path: "/w/a.rs".to_owned(),
            offset: Some(10),
            limit: Some(20),
        })
    );
}

/// What the preview keeps has a ceiling: an edit can be a whole file.
#[test]
fn a_big_edit_is_kept_only_in_part() {
    let input = json!({ "file_path": "/w/a.rs", "old_string": "x".repeat(KEPT * 3) });
    let Some(Touch::Edit { before, .. }) = touch_of("Edit", &input) else {
        panic!("an edit");
    };
    assert_eq!(before.chars().count(), KEPT + 1);
}

#[test]
fn a_tool_the_preview_cannot_draw_has_no_touch() {
    assert_eq!(touch_of("WebFetch", &json!({ "url": "https://x" })), None);
    assert_eq!(touch_of("Edit", &json!({ "new_string": "no path" })), None);
}

/// A question step carries the question and its choices: the session waits
/// on the person, and "nothing to show" was the island saying nothing about
/// what they were being asked.
#[test]
fn a_question_step_says_what_is_asked_and_the_choices() {
    let input = serde_json::json!({
        "questions": [{
            "question": "Where does the viewer run?",
            "header": "Viewer",
            "multiSelect": false,
            "options": [
                { "label": "Web, PWA later", "description": "…" },
                { "label": "Native app", "description": "…" }
            ]
        }]
    });
    assert_eq!(
        target_of(&input).as_deref(),
        Some("Where does the viewer run?")
    );
    let Some(Touch::Ask { questions }) = touch_of("AskUserQuestion", &input) else {
        panic!("a question step");
    };
    assert_eq!(questions.len(), 1);
    assert_eq!(questions[0].header.as_deref(), Some("Viewer"));
    assert_eq!(questions[0].options, ["Web, PWA later", "Native app"]);
    assert!(!questions[0].multi);
    assert_eq!(touch_of("AskUserQuestion", &serde_json::json!({})), None);
}

#[test]
fn a_plan_step_carries_the_plan() {
    let input = serde_json::json!({ "plan": "1. Spike\n2. Ship" });
    assert_eq!(
        touch_of("ExitPlanMode", &input),
        Some(Touch::Plan {
            plan: "1. Spike\n2. Ship".to_owned()
        })
    );
}
