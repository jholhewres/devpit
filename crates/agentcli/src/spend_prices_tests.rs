use super::*;
use crate::spend_scan::Record;

#[test]
fn each_model_id_finds_its_family() {
    let input = |model: &str| rates_for(model).map(|rates| rates.input);
    assert_eq!(input("claude-fable-5-1"), Some(10.0));
    assert_eq!(input("claude-opus-5"), Some(5.0));
    assert_eq!(input("claude-sonnet-5"), Some(2.0));
    assert_eq!(input("claude-opus-4-8"), Some(5.0));
    assert_eq!(input("anthropic/claude-opus-4.6"), Some(5.0));
    assert_eq!(input("claude-opus-4-1-20250805"), Some(15.0));
    assert_eq!(input("claude-opus-4-20250514"), Some(15.0));
    assert_eq!(input("claude-opus-4-9"), Some(5.0));
    assert_eq!(input("claude-sonnet-4-5-20250929"), Some(3.0));
    assert_eq!(input("claude-haiku-4-5-20251001"), Some(1.0));
    assert_eq!(input("claude-3-5-haiku"), None);
    assert_eq!(input("claude-haiku-3-5"), Some(0.8));
    assert_eq!(input("glm-4.6"), None);
    assert_eq!(input("<synthetic>"), None);
}

#[test]
fn a_message_costs_its_tokens_at_their_own_rates() {
    let record = Record {
        key: "m:r".to_owned(),
        at: 0,
        session_id: "s".to_owned(),
        cwd: "/w".to_owned(),
        model: "claude-opus-5".to_owned(),
        input: 1_000_000,
        output: 1_000_000,
        cache_read: 1_000_000,
        cache_write_5m: 1_000_000,
        cache_write_1h: 1_000_000,
    };
    // 5 + 25 + 0.5 + 6.25 + 10
    assert!((cost_of(&record).expect("priced") - 46.75).abs() < 1e-9);
    let unknown = Record {
        model: "glm-4.6".to_owned(),
        ..record
    };
    assert_eq!(cost_of(&unknown), None);
}
