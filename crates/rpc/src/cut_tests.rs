use super::*;
use crate::chat::{CallState, Role};

fn result(output: String) -> Part {
    Part::ToolResult {
        call_id: "call_1".to_owned(),
        output,
        is_error: false,
        parent: None,
    }
}

fn output_of(part: &Part) -> &str {
    let Part::ToolResult { output, .. } = part else {
        panic!("a tool result");
    };
    output
}

#[test]
fn a_short_output_goes_whole() {
    let small = "test passed\n".repeat(100);
    assert!(small.len() <= WHOLE);
    assert_eq!(cut_text(&small), None);
    assert_eq!(cut_part(&result(small.clone())), None);
    // A text block inside a long output is cut only when it is long itself.
    let blocks = serde_json::json!([
        { "type": "text", "text": small },
        { "type": "image", "source": { "media_type": "image/png", "data": "A".repeat(8_000) } },
    ])
    .to_string();
    let cut = cut_part(&result(blocks)).expect("the image is named");
    let kept: Value = serde_json::from_str(output_of(&cut)).expect("blocks");
    assert_eq!(kept[0]["text"], small);
}

#[test]
fn a_long_text_keeps_its_head_and_tail_and_says_how_long_it_was() {
    let long = format!(
        "{}{}{}",
        "a".repeat(HEAD),
        "m".repeat(10_000),
        "z".repeat(TAIL)
    );
    let cut = cut_part(&result(long.clone())).expect("cut");
    let said = output_of(&cut);
    assert!(said.starts_with(&"a".repeat(HEAD)));
    assert!(said.ends_with(&"z".repeat(TAIL)));
    assert!(!said.contains('m'));
    assert!(said.contains(&format!("{CUT_MARK}{} bytes in all]", long.len())));
    assert!(said.len() < WHOLE);
}

#[test]
fn a_cut_inside_a_character_moves_to_its_edge() {
    // Three-byte characters, so byte HEAD lands inside one and so does the tail.
    let long = "€".repeat(WHOLE);
    assert!(!long.is_char_boundary(HEAD) || !long.is_char_boundary(long.len() - TAIL));
    let said = cut_text(&long).expect("cut");
    // Valid UTF-8 by construction of `String`; the head and tail are whole euros.
    let (head, rest) = said.split_once('\n').expect("head");
    assert!(head.chars().all(|c| c == '€'));
    let tail = rest.rsplit_once('\n').expect("tail").1;
    assert!(tail.chars().all(|c| c == '€'));
}

#[test]
fn a_small_image_loses_its_data_too() {
    let output = serde_json::json!([
        { "type": "image", "source": { "type": "base64", "media_type": "image/jpeg", "data": "/9j/AAAA" } },
    ])
    .to_string();
    assert!(output.len() < WHOLE);
    let cut = cut_part(&result(output)).expect("named");
    assert!(!output_of(&cut).contains("/9j/"));
}

#[test]
fn an_image_is_named_with_its_size_and_loses_its_data() {
    let data = "A".repeat(400_000);
    let output = serde_json::json!([
        { "type": "image", "source": { "type": "base64", "media_type": "image/png", "data": data } },
        { "type": "text", "text": "the screen" },
    ])
    .to_string();
    let cut = cut_part(&result(output)).expect("cut");
    let blocks: Value = serde_json::from_str(output_of(&cut)).expect("still blocks");
    assert_eq!(
        blocks[0],
        serde_json::json!({ "type": "image", "mime": "image/png", "bytes": 300_000, "cut": true })
    );
    assert_eq!(blocks[1]["text"], "the screen");
    assert!(output_of(&cut).len() < 200);
}

#[test]
fn a_long_text_block_beside_an_image_is_cut_on_its_own() {
    let output = serde_json::json!([
        { "type": "text", "text": "x".repeat(20_000) },
        { "type": "image", "source": { "media_type": "image/png", "data": "A".repeat(8_000) } },
    ])
    .to_string();
    let cut = cut_part(&result(output)).expect("cut");
    let blocks: Value = serde_json::from_str(output_of(&cut)).expect("still blocks");
    assert!(blocks[0]["text"].as_str().expect("text").contains(CUT_MARK));
    assert_eq!(blocks[1]["cut"], true);
}

#[test]
fn a_long_json_array_that_is_not_blocks_is_cut_as_text() {
    let numbers = serde_json::to_string(&(0..10_000).collect::<Vec<_>>()).expect("json");
    assert!(numbers.len() > WHOLE);
    let cut = cut_part(&result(numbers.clone())).expect("cut");
    assert!(output_of(&cut).contains(&format!("{CUT_MARK}{} bytes in all]", numbers.len())));
    let small = serde_json::json!([{ "type": "text", "text": "ok" }]).to_string();
    assert_eq!(cut_part(&result(small)), None);
}

#[test]
fn an_image_beside_a_large_block_of_another_kind_does_not_carry_it_whole() {
    let output = serde_json::json!([
        { "type": "image", "source": { "media_type": "image/png", "data": "AAAA" } },
        { "type": "resource", "resource": { "uri": "file:///a", "blob": "A".repeat(100_000) } },
    ])
    .to_string();
    let cut = cut_part(&result(output)).expect("cut");
    assert!(output_of(&cut).len() < WHOLE);
    assert!(!output_of(&cut).contains(&"A".repeat(5_000)));
}

#[test]
fn an_array_with_an_untyped_item_is_cut_as_text_so_the_window_sees_the_cut() {
    let output = serde_json::json!([
        { "type": "text", "text": "x".repeat(20_000) },
        "loose",
    ])
    .to_string();
    let cut = cut_part(&result(output.clone())).expect("cut");
    assert!(output_of(&cut).contains(&format!("\n{CUT_MARK}{} bytes in all]\n", output.len())));
}

#[test]
fn long_blocks_with_nothing_to_name_or_cut_are_cut_as_text() {
    let output = serde_json::json!([
        { "type": "resource", "resource": { "uri": "file:///a", "text": "r".repeat(20_000) } },
    ])
    .to_string();
    let cut = cut_part(&result(output.clone())).expect("cut");
    assert!(output_of(&cut).len() < WHOLE);
}

#[test]
fn a_tool_call_and_the_answer_go_whole() {
    let call = Part::ToolCall {
        id: "call_1".to_owned(),
        name: "Edit".to_owned(),
        input: "y".repeat(50_000),
        state: CallState::Ok,
        parent: None,
    };
    assert_eq!(cut_part(&call), None);
    let said = Part::Text {
        text: "z".repeat(50_000),
        parent: None,
    };
    assert_eq!(cut_part(&said), None);
}

#[test]
fn a_message_is_cut_part_by_part() {
    let mut message = Message {
        id: "m".to_owned(),
        turn_id: None,
        role: Role::Assistant,
        parts: vec![result("w".repeat(9_000)), result("ok".to_owned())],
        created_at: 0.0,
        streaming: false,
    };
    cut_message(&mut message);
    assert!(output_of(&message.parts[0]).contains(CUT_MARK));
    assert_eq!(output_of(&message.parts[1]), "ok");
}

/// The real transcript that froze the window, cut as `chat_history` cuts it.
/// Local only: `CUT_FIXTURE=<path to a .jsonl>`; the file is never versioned.
#[test]
#[ignore = "needs CUT_FIXTURE, a transcript kept outside the repository"]
fn a_real_transcript_crosses_under_thirteen_megabytes() {
    let path = std::env::var("CUT_FIXTURE").expect("CUT_FIXTURE");
    let text = std::fs::read_to_string(&path).expect("the fixture");
    let mut whole = 0;
    let mut cut = 0;
    let mut images = 0;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let mut message: Message = serde_json::from_str(line).expect("a message");
        whole += serde_json::to_string(&message).expect("json").len();
        cut_message(&mut message);
        let said = serde_json::to_string(&message).expect("json");
        images += said.matches(r#"\"cut\":true"#).count();
        for part in &message.parts {
            let Part::ToolResult { output, .. } = part else {
                continue;
            };
            let Ok(Value::Array(blocks)) = serde_json::from_str::<Value>(output) else {
                continue;
            };
            assert!(
                blocks
                    .iter()
                    .all(|block| block["type"] != "image" || block.get("source").is_none()),
                "an image's data crossed"
            );
        }
        cut += said.len();
    }
    eprintln!("whole {whole} bytes, cut {cut} bytes, {images} images named");
    assert!(cut < 13_000_000, "{cut} bytes");
    assert!(images > 0);
}
