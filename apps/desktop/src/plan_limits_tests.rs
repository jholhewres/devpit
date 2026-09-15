use serde_json::json;

use super::*;

#[test]
fn the_sign_in_is_read_from_the_credentials_and_its_absence_said() {
    let Ok(signed) = sign_in_of(
        r#"{"mcpOAuth":{},"claudeAiOauth":{"accessToken":"t-1","subscriptionType":"max","rateLimitTier":"default_claude_max_5x"}}"#,
    ) else {
        panic!("signed in");
    };
    assert_eq!(signed.token, "t-1");
    assert_eq!(
        plan_label(signed.subscription.as_deref(), signed.tier.as_deref()).as_deref(),
        Some("Max (5x)")
    );
    assert_eq!(plan_label(Some("pro"), None).as_deref(), Some("Pro"));
    assert!(sign_in_of(r#"{"mcpOAuth":{}}"#).is_err());
    assert!(sign_in_of("not json").is_err());
}

#[test]
fn the_limits_array_is_read_first() {
    let body = json!({
        "limits": [
            {"kind": "session", "percent": 42.5, "resets_at": "2026-09-15T15:00:00Z"},
            {"kind": "weekly_all", "percent": 130, "resets_at": "2026-09-20T00:00:00+00:00"},
            {"kind": "weekly_scoped", "percent": 10, "scope": {"model": {"display_name": "Opus"}}},
            {"kind": "overage", "percent": 1}
        ],
        "five_hour": {"utilization": 99}
    });
    let windows = windows_of(&body);
    let labels: Vec<&str> = windows.iter().map(|one| one.label.as_str()).collect();
    assert_eq!(labels, ["5-hour", "Weekly", "Weekly · Opus"]);
    assert_eq!(windows[0].percent, 42.5);
    assert_eq!(windows[1].percent, 100.0, "clamped");
    assert_eq!(
        windows[0].resets_at,
        epoch_of("2026-09-15T15:00:00Z").map(|at| at as f64)
    );
    assert_eq!(windows[2].resets_at, None);
}

#[test]
fn an_array_with_no_window_this_screen_draws_falls_back_to_the_flat_fields() {
    let body = json!({
        "limits": [{"kind": "overage", "percent": 1}],
        "five_hour": {"utilization": 20.0}
    });
    let windows = windows_of(&body);
    assert_eq!(windows.len(), 1);
    assert_eq!(
        (windows[0].label.as_str(), windows[0].percent),
        ("5-hour", 20.0)
    );
}

#[test]
fn the_older_flat_fields_are_read_when_there_is_no_array() {
    let body = json!({
        "five_hour": {"utilization": 12.0, "resets_at": "2026-09-15T15:00:00Z"},
        "seven_day": {"utilization": 55.0, "resets_at": null},
        "seven_day_sonnet": {"utilization": 3.0}
    });
    let labels: Vec<(String, f64)> = windows_of(&body)
        .into_iter()
        .map(|one| (one.label, one.percent))
        .collect();
    assert_eq!(
        labels,
        [
            ("5-hour".to_owned(), 12.0),
            ("Weekly".to_owned(), 55.0),
            ("Weekly · Sonnet".to_owned(), 3.0)
        ]
    );
}
