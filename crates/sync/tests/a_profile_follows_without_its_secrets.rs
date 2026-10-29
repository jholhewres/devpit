//! An agent profile made on one machine shows up on the other with what it
//! is — its name, agent, arguments, models and which variables it sets —
//! and none of its secrets: those are asked for on each machine.

mod support;

use devpit_core::preference as key;
use devpit_sync::clock::Clock;
use devpit_sync::{files, State, SyncKey};
use serde_json::{json, Value};
use support::{opened, Home, Stand};

fn home(id: &str, made: &SyncKey) -> Home {
    let home = Home::new(id);
    files::keep_key(&home.root(), made).expect("keep");
    files::keep_state(
        &home.root(),
        &State {
            machine: id.to_owned(),
            clock: Clock::new(id),
            spaces: Default::default(),
            synced_at: None,
        },
    )
    .expect("state");
    home
}

fn profiles(home: &Home) -> Vec<Value> {
    let raw = home.pref(key::AGENT_PROFILES).unwrap_or_default();
    serde_json::from_str(&raw).unwrap_or_default()
}

fn set_profiles(home: &Home, list: Value) {
    home.store
        .set_preference(key::AGENT_PROFILES, &list.to_string())
        .expect("profiles");
}

fn env_of(profile: &Value, name: &str) -> Option<String> {
    profile["env"]
        .as_array()?
        .iter()
        .find(|var| var["name"] == name)
        .and_then(|var| var["value"].as_str().map(str::to_owned))
}

async fn rounds(homes: &mut [Home], stand: &Stand, times: usize) {
    for _ in 0..times {
        for one in homes.iter_mut() {
            one.sync(stand).await.expect("round");
        }
    }
}

fn glm(home: &Home) -> Value {
    let config = home.root().join(".claude-glm");
    json!([{
        "id": "01KGLMPROFILE",
        "label": "GLM",
        "base": "claude",
        "command": "claude",
        "args": ["--permission-mode", "bypassPermissions"],
        "env": [
            {"name": "ANTHROPIC_BASE_URL", "value": "https://api.z.ai/api/anthropic"},
            {"name": "ANTHROPIC_AUTH_TOKEN", "value": "sk-SECRETONA"},
            {"name": "CLAUDE_CONFIG_DIR", "value": config.to_string_lossy()},
            {"name": "ANTHROPIC_MODEL", "value": "glm-5"}
        ],
        "models": ["glm-5"]
    }])
}

#[tokio::test]
async fn a_profile_arrives_without_its_secrets_and_keeps_the_one_filled_here() {
    let stand = Stand::default();
    let made = SyncKey::generate().expect("key");
    let mut homes = [home("m_a", &made), home("m_b", &made)];
    set_profiles(&homes[0], glm(&homes[0]));

    rounds(&mut homes, &stand, 3).await;

    let arrived = profiles(&homes[1]);
    assert_eq!(arrived.len(), 1, "{arrived:?}");
    let one = &arrived[0];
    assert_eq!(one["id"], "01KGLMPROFILE");
    assert_eq!(one["label"], "GLM");
    assert_eq!(one["base"], "claude");
    assert_eq!(one["command"], "claude");
    assert_eq!(
        one["args"],
        json!(["--permission-mode", "bypassPermissions"])
    );
    assert_eq!(one["models"], json!(["glm-5"]));
    assert_eq!(
        env_of(one, "ANTHROPIC_BASE_URL").as_deref(),
        Some("https://api.z.ai/api/anthropic")
    );
    assert_eq!(env_of(one, "ANTHROPIC_MODEL").as_deref(), Some("glm-5"));
    assert_eq!(
        env_of(one, "ANTHROPIC_AUTH_TOKEN").as_deref(),
        Some(""),
        "named, so this machine knows to ask, and empty"
    );
    assert_eq!(
        env_of(one, "CLAUDE_CONFIG_DIR"),
        Some(
            homes[1]
                .root()
                .join(".claude-glm")
                .to_string_lossy()
                .into_owned()
        ),
        "under this machine's home"
    );

    // B fills its own token; neither machine's token reaches the other.
    let mut filled = profiles(&homes[1]);
    for var in filled[0]["env"].as_array_mut().expect("env") {
        if var["name"] == "ANTHROPIC_AUTH_TOKEN" {
            var["value"] = json!("sk-SECRETONB");
        }
    }
    set_profiles(&homes[1], Value::Array(filled));
    rounds(&mut homes, &stand, 3).await;

    assert_eq!(
        env_of(&profiles(&homes[0])[0], "ANTHROPIC_AUTH_TOKEN").as_deref(),
        Some("sk-SECRETONA")
    );
    assert_eq!(
        env_of(&profiles(&homes[1])[0], "ANTHROPIC_AUTH_TOKEN").as_deref(),
        Some("sk-SECRETONB")
    );
    let sealed = opened(&made, &stand.kept());
    assert!(!sealed.contains("SECRETON"), "a token synced");
    for one in &homes {
        assert!(
            !sealed.contains(one.root().to_string_lossy().as_ref()),
            "a home path synced"
        );
    }
}

#[tokio::test]
async fn a_program_named_by_its_path_stays_on_its_machine() {
    let stand = Stand::default();
    let made = SyncKey::generate().expect("key");
    let mut homes = [home("m_a", &made), home("m_b", &made)];
    let mut list = glm(&homes[0]);
    list[0]["command"] = json!("/opt/claude/bin/claude");
    set_profiles(&homes[0], list);

    rounds(&mut homes, &stand, 3).await;

    assert_eq!(profiles(&homes[1])[0]["command"], "");
    assert!(!opened(&made, &stand.kept()).contains("/opt/claude"));
}

#[tokio::test]
async fn a_base_url_changed_elsewhere_waits_for_the_person_here() {
    let stand = Stand::default();
    let made = SyncKey::generate().expect("key");
    let mut homes = [home("m_a", &made), home("m_b", &made)];
    set_profiles(&homes[0], glm(&homes[0]));
    rounds(&mut homes, &stand, 3).await;

    let mut moved = profiles(&homes[0]);
    for var in moved[0]["env"].as_array_mut().expect("env") {
        if var["name"] == "ANTHROPIC_BASE_URL" {
            var["value"] = json!("https://elsewhere.example/api");
        }
    }
    set_profiles(&homes[0], Value::Array(moved));
    rounds(&mut homes, &stand, 3).await;

    let here = &profiles(&homes[1])[0];
    assert_eq!(
        env_of(here, "ANTHROPIC_BASE_URL").as_deref(),
        Some("https://api.z.ai/api/anthropic"),
        "the token is not sent somewhere new without the person"
    );
    let waiting: Value = serde_json::from_str(
        &homes[1]
            .pref(key::AGENT_PROFILES_PENDING)
            .unwrap_or_default(),
    )
    .expect("pending");
    assert_eq!(
        waiting,
        json!({"01KGLMPROFILE": {"ANTHROPIC_BASE_URL": "https://elsewhere.example/api"}})
    );
    for one in homes.iter_mut() {
        let outcome = one.sync(&stand).await.expect("round");
        let mentions = |list: &[String]| list.iter().any(|space| space == "profiles");
        assert!(
            !mentions(&outcome.sent) && !mentions(&outcome.written),
            "{outcome:?}"
        );
    }
}
