use serde_json::json;

use super::*;

#[test]
fn a_board_tool_reads_a_card_sent_as_card_or_epic() {
    assert_eq!(
        aliased("comment", json!({ "card": "card_1", "body": "hi" })),
        json!({ "cardId": "card_1", "body": "hi" })
    );
    assert_eq!(
        aliased("card", json!({ "epic": "card_2" })),
        json!({ "cardId": "card_2" })
    );
    // The key it reads wins; nothing is overwritten.
    assert_eq!(
        aliased("card", json!({ "cardId": "card_1", "card": "card_2" })),
        json!({ "cardId": "card_1", "card": "card_2" })
    );
}

/// Devflow's tools refuse a field they do not take: the alias is moved, not
/// left beside the key, or `cardId` would be refused after all.
#[test]
fn devflows_strict_tools_take_a_card_sent_as_card_id() {
    let gate = devpit_devflow::gate::asked(&aliased("devflow_gate", json!({ "cardId": "c1" })))
        .expect("the gate takes it");
    assert_eq!(gate.card.as_deref(), Some("c1"));
    let review =
        devpit_devflow::review::asked(&aliased("devflow_review", json!({ "cardId": "c1" })))
            .expect("the review takes it");
    assert_eq!(review.card, "c1");
    let evidence =
        devpit_devflow::evidence::asked(&aliased("devflow_evidence", json!({ "cardId": "c1" })))
            .expect("the evidence takes it");
    assert_eq!(evidence.card, "c1");
    assert_eq!(
        aliased("devflow_plan", json!({ "cardId": "c1", "cards": [] })),
        json!({ "epic": "c1", "cards": [] })
    );
}

#[test]
fn a_session_is_named_by_name_or_session_where_a_tool_acts_on_one() {
    for method in ["screen", "stop", "transcript", "draft", "send_draft"] {
        assert_eq!(
            aliased(method, json!({ "session": "fix-login" })),
            json!({ "name": "fix-login" }),
            "{method}"
        );
    }
    // Resume's and start's `name` is the new name, never the target.
    for method in ["resume", "start"] {
        let asked = json!({ "session": "s-1", "name": "again" });
        assert_eq!(
            aliased(method, asked.clone())["name"],
            asked["name"],
            "{method}"
        );
    }
    assert_eq!(
        aliased("resume", json!({ "session": "s-1" })),
        json!({ "session": "s-1" })
    );
}

#[test]
fn an_empty_key_is_filled_and_a_method_without_a_card_is_left_alone() {
    assert_eq!(
        aliased("card", json!({ "cardId": "", "card": "card_1" })),
        json!({ "cardId": "card_1" })
    );
    assert_eq!(
        aliased("board", json!({ "card": "x" })),
        json!({ "card": "x" })
    );
    assert_eq!(aliased("card", Value::Null), Value::Null);
}
