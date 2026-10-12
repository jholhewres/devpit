use serde_json::{json, Value};

use super::{allowed, ask, clients, cut_to, READS};
use crate::remote_devices::Devices;

#[test]
fn nothing_but_a_read_is_answered_from_outside() {
    for method in crate::agent_api::METHODS
        .iter()
        .filter(|method| !READS.contains(method))
    {
        let refused = ask(method, &json!({ "cardId": "c1", "body": "x" }));
        assert!(
            refused.is_err_and(|why| why.contains("not read from outside")),
            "{method}"
        );
    }
}

#[test]
fn every_tool_the_allowlist_lets_through_lands_on_a_read() {
    for name in devpit_agentapi::remote_mcp::ALLOWED {
        let line = json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": name } })
            .to_string();
        let asked = std::cell::RefCell::new(None);
        devpit_agentapi::remote_mcp::handle(&line, &|method, _| {
            *asked.borrow_mut() = Some(method.to_owned());
            Ok(Value::Null)
        });
        let method = asked.into_inner().expect("asked of the app");
        assert!(READS.contains(&method.as_str()), "{name} → {method}");
    }
}

#[test]
fn remote_mcp_is_off_until_the_person_turns_it_on() {
    let dir = tempfile::tempdir().expect("dir");
    let store = devpit_core::Store::open(&dir.path().join("state.db")).expect("open");
    assert!(!allowed(&store));
    store
        .set_preference(devpit_core::preference::REMOTE_MCP, "true")
        .expect("set");
    assert!(allowed(&store));
    store
        .set_preference(devpit_core::preference::REMOTE_MCP, "false")
        .expect("set");
    assert!(!allowed(&store));
}

#[test]
fn turning_it_off_cuts_the_mcp_clients_and_leaves_the_phones() {
    let dir = tempfile::tempdir().expect("dir");
    let mut devices = Devices::default();
    devices
        .pair_key("Pixel", &[1u8; 32], false, 1.0)
        .expect("phone");
    let laptop = devices
        .pair_key("laptop", &[2u8; 32], true, 1.0)
        .expect("laptop");
    devices.write(dir.path()).expect("write");
    assert_eq!(clients(dir.path()), [laptop.id]);
}

#[test]
fn long_text_is_cut_on_a_character() {
    assert_eq!(cut_to("abc", 5), "abc");
    assert_eq!(cut_to("ação", 2), "a…");
}
