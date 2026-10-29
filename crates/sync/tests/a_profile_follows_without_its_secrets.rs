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
        json!([]),
        "arguments stay where they were typed"
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
    take(&homes[1], "01KGLMPROFILE");

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
    assert_eq!(
        asked(&homes[1], "01KGLMPROFILE"),
        json!({"env.ANTHROPIC_BASE_URL": "https://elsewhere.example/api"})
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

/// What waits for the person's yes on this machine, by field.
fn asked(home: &Home, id: &str) -> Value {
    let kept: Value =
        serde_json::from_str(&home.pref(key::AGENT_PROFILES_SYNCED).unwrap_or_default())
            .unwrap_or_default();
    let mut asked = serde_json::Map::new();
    if let Some(shadow) = kept["profiles"][id]["shadow"].as_object() {
        for (field, held) in shadow {
            if held["ask"] == true {
                asked.insert(field.clone(), held["value"].clone());
            }
        }
    }
    Value::Object(asked)
}

fn arrived(home: &Home, id: &str) -> bool {
    let kept: Value =
        serde_json::from_str(&home.pref(key::AGENT_PROFILES_SYNCED).unwrap_or_default())
            .unwrap_or_default();
    kept["profiles"][id]["arrived"] == true
}

fn edit(home: &Home, change: impl FnOnce(&mut Value)) {
    let mut list = Value::Array(profiles(home));
    change(&mut list[0]);
    set_profiles(home, list);
}

fn set_env(profile: &mut Value, name: &str, value: &str) {
    let env = profile["env"].as_array_mut().expect("env");
    match env.iter_mut().find(|var| var["name"] == name) {
        Some(var) => var["value"] = json!(value),
        None => env.push(json!({"name": name, "value": value})),
    }
}

#[tokio::test]
async fn a_profile_deleted_here_stays_deleted_here_and_everywhere() {
    let stand = Stand::default();
    let made = SyncKey::generate().expect("key");
    let mut homes = [home("m_a", &made), home("m_b", &made)];
    set_profiles(&homes[0], glm(&homes[0]));
    rounds(&mut homes, &stand, 3).await;
    assert_eq!(profiles(&homes[1]).len(), 1);

    set_profiles(&homes[0], json!([]));
    rounds(&mut homes, &stand, 3).await;

    assert!(profiles(&homes[0]).is_empty(), "{:?}", profiles(&homes[0]));
    assert!(profiles(&homes[1]).is_empty(), "{:?}", profiles(&homes[1]));
}

#[tokio::test]
async fn a_variable_removed_here_stays_removed() {
    let stand = Stand::default();
    let made = SyncKey::generate().expect("key");
    let mut homes = [home("m_a", &made), home("m_b", &made)];
    set_profiles(&homes[0], glm(&homes[0]));
    rounds(&mut homes, &stand, 3).await;

    edit(&homes[0], |profile| {
        profile["env"]
            .as_array_mut()
            .expect("env")
            .retain(|var| var["name"] != "ANTHROPIC_MODEL")
    });
    rounds(&mut homes, &stand, 3).await;

    for one in &homes {
        assert_eq!(env_of(&profiles(one)[0], "ANTHROPIC_MODEL"), None);
    }
}

#[tokio::test]
async fn a_base_url_added_elsewhere_waits_too() {
    let stand = Stand::default();
    let made = SyncKey::generate().expect("key");
    let mut homes = [home("m_a", &made), home("m_b", &made)];
    let mut list = glm(&homes[0]);
    list[0]["env"] = json!([{"name": "ANTHROPIC_API_KEY", "value": "sk-SECRETONA"}]);
    set_profiles(&homes[0], list);
    rounds(&mut homes, &stand, 3).await;
    take(&homes[1], "01KGLMPROFILE");

    edit(&homes[0], |profile| {
        set_env(
            profile,
            "ANTHROPIC_BASE_URL",
            "https://elsewhere.example/api",
        )
    });
    rounds(&mut homes, &stand, 3).await;

    assert_eq!(env_of(&profiles(&homes[1])[0], "ANTHROPIC_BASE_URL"), None);
    assert_eq!(
        asked(&homes[1], "01KGLMPROFILE"),
        json!({"env.ANTHROPIC_BASE_URL": "https://elsewhere.example/api"})
    );
}

#[tokio::test]
async fn a_program_changed_elsewhere_waits_for_the_person_here() {
    let stand = Stand::default();
    let made = SyncKey::generate().expect("key");
    let mut homes = [home("m_a", &made), home("m_b", &made)];
    set_profiles(&homes[0], glm(&homes[0]));
    rounds(&mut homes, &stand, 3).await;
    take(&homes[1], "01KGLMPROFILE");

    edit(&homes[0], |profile| profile["command"] = json!("sh"));
    rounds(&mut homes, &stand, 3).await;

    assert_eq!(profiles(&homes[1])[0]["command"], "claude");
    assert_eq!(asked(&homes[1], "01KGLMPROFILE"), json!({"command": "sh"}));
}

#[tokio::test]
async fn a_profile_from_elsewhere_waits_for_a_yes_before_it_runs() {
    let stand = Stand::default();
    let made = SyncKey::generate().expect("key");
    let mut homes = [home("m_a", &made), home("m_b", &made)];
    set_profiles(&homes[0], glm(&homes[0]));

    rounds(&mut homes, &stand, 3).await;

    assert!(arrived(&homes[1], "01KGLMPROFILE"));
    assert!(
        !arrived(&homes[0], "01KGLMPROFILE"),
        "made here, so already the person's"
    );
}

#[tokio::test]
async fn a_config_dir_outside_home_stays_here() {
    let stand = Stand::default();
    let made = SyncKey::generate().expect("key");
    let mut homes = [home("m_a", &made), home("m_b", &made)];
    set_profiles(&homes[0], glm(&homes[0]));
    rounds(&mut homes, &stand, 3).await;

    edit(&homes[1], |profile| {
        set_env(profile, "CLAUDE_CONFIG_DIR", "/data/claude-glm")
    });
    rounds(&mut homes, &stand, 3).await;

    assert_eq!(
        env_of(&profiles(&homes[1])[0], "CLAUDE_CONFIG_DIR").as_deref(),
        Some("/data/claude-glm")
    );
    assert!(!opened(&made, &stand.kept()).contains("/data/claude-glm"));
    for one in homes.iter_mut() {
        let outcome = one.sync(&stand).await.expect("round");
        assert!(
            !outcome.written.iter().any(|space| space == "profiles"),
            "{outcome:?}"
        );
    }
}

#[tokio::test]
async fn a_secret_under_a_shared_looking_name_and_credentials_in_a_url_stay_here() {
    let stand = Stand::default();
    let made = SyncKey::generate().expect("key");
    let mut homes = [home("m_a", &made), home("m_b", &made)];
    let mut list = glm(&homes[0]);
    list[0]["env"] = json!([
        {"name": "AZURE_MODEL_API_KEY", "value": "SECRETMODELKEY"},
        {"name": "GATEWAY_BASE_URL", "value": "https://jo:SECRETINURL@gw.example/v1"},
        {"name": "OTHER_BASE_URL", "value": "https://gw.example/v1?api-key=SECRETINQUERY"}
    ]);
    set_profiles(&homes[0], list);

    rounds(&mut homes, &stand, 3).await;

    let sealed = opened(&made, &stand.kept());
    for secret in ["SECRETMODELKEY", "SECRETINURL", "SECRETINQUERY"] {
        assert!(!sealed.contains(secret), "{secret} synced");
    }
}

/// The person's yes here to a profile that arrived from another machine.
fn take(home: &Home, id: &str) {
    let mut kept: Value =
        serde_json::from_str(&home.pref(key::AGENT_PROFILES_SYNCED).unwrap_or_default())
            .unwrap_or_default();
    kept["profiles"][id]["arrived"] = json!(false);
    home.store
        .set_preference(key::AGENT_PROFILES_SYNCED, &kept.to_string())
        .expect("taken");
}

/// A no to a base URL this machine never had stays a no: the other
/// machine's value sits beside, unused, and the round does not ask again.
#[tokio::test]
async fn a_base_url_kept_out_here_is_not_asked_again() {
    let stand = Stand::default();
    let made = SyncKey::generate().expect("key");
    let mut homes = [home("m_a", &made), home("m_b", &made)];
    let mut list = glm(&homes[0]);
    list[0]["env"] = json!([{"name": "ANTHROPIC_API_KEY", "value": "sk-SECRETONA"}]);
    set_profiles(&homes[0], list);
    rounds(&mut homes, &stand, 3).await;
    take(&homes[1], "01KGLMPROFILE");
    edit(&homes[0], |profile| {
        set_env(
            profile,
            "ANTHROPIC_BASE_URL",
            "https://elsewhere.example/api",
        )
    });
    rounds(&mut homes, &stand, 3).await;

    // "Keep this one", as the person answers it here.
    let mut kept = devpit_sync::profiles::kept(&homes[1].store);
    let held = kept.profiles.get_mut("01KGLMPROFILE").expect("held");
    devpit_sync::profiles::keep_out(held, "env.ANTHROPIC_BASE_URL", false);
    devpit_sync::profiles::keep(&homes[1].store, &kept).expect("kept");
    rounds(&mut homes, &stand, 2).await;

    assert_eq!(asked(&homes[1], "01KGLMPROFILE"), json!({}));
    assert_eq!(env_of(&profiles(&homes[1])[0], "ANTHROPIC_BASE_URL"), None);
}

#[tokio::test]
async fn a_config_folder_changed_elsewhere_waits_too() {
    let stand = Stand::default();
    let made = SyncKey::generate().expect("key");
    let mut homes = [home("m_a", &made), home("m_b", &made)];
    set_profiles(&homes[0], glm(&homes[0]));
    rounds(&mut homes, &stand, 3).await;
    take(&homes[1], "01KGLMPROFILE");

    let other = homes[0].root().join(".claude-other");
    edit(&homes[0], |profile| {
        set_env(profile, "CLAUDE_CONFIG_DIR", &other.to_string_lossy())
    });
    rounds(&mut homes, &stand, 3).await;

    assert_eq!(
        env_of(&profiles(&homes[1])[0], "CLAUDE_CONFIG_DIR"),
        Some(
            homes[1]
                .root()
                .join(".claude-glm")
                .to_string_lossy()
                .into_owned()
        ),
        "the account this profile spends stays until the person says"
    );
    assert_eq!(
        asked(&homes[1], "01KGLMPROFILE"),
        json!({"env.CLAUDE_CONFIG_DIR": "~/.claude-other"})
    );
}

#[tokio::test]
async fn a_removed_profile_settles_and_stops_writing() {
    let stand = Stand::default();
    let made = SyncKey::generate().expect("key");
    let mut homes = [home("m_a", &made), home("m_b", &made)];
    set_profiles(&homes[0], glm(&homes[0]));
    rounds(&mut homes, &stand, 3).await;
    set_profiles(&homes[0], json!([]));
    rounds(&mut homes, &stand, 3).await;

    for one in homes.iter_mut() {
        let outcome = one.sync(&stand).await.expect("round");
        let mentions = |list: &[String]| list.iter().any(|space| space == "profiles");
        assert!(
            !mentions(&outcome.sent) && !mentions(&outcome.written),
            "{outcome:?}"
        );
    }
}

#[tokio::test]
async fn a_profile_a_step_here_uses_is_not_deleted_from_elsewhere() {
    let stand = Stand::default();
    let made = SyncKey::generate().expect("key");
    let mut homes = [home("m_a", &made), home("m_b", &made)];
    set_profiles(&homes[0], glm(&homes[0]));
    rounds(&mut homes, &stand, 3).await;
    let board = homes[1]
        .store
        .add_project(&homes[1].root().join("board"), None)
        .expect("project");
    homes[1]
        .store
        .create_step(
            &board,
            "agent",
            "Review",
            r#"{"profile":"01KGLMPROFILE"}"#,
            false,
        )
        .expect("step");

    set_profiles(&homes[0], json!([]));
    rounds(&mut homes, &stand, 3).await;

    assert_eq!(
        profiles(&homes[1]).len(),
        1,
        "a step here still runs under it"
    );
    assert!(
        devpit_sync::profiles::kept(&homes[1].store)
            .profiles
            .get("01KGLMPROFILE")
            .is_some_and(|held| held.arrived),
        "and it still waits for a yes before it runs"
    );
}

/// A profile that arrived and was not yet taken still asks before a change
/// elsewhere to where its token goes: the person takes what they were shown.
#[tokio::test]
async fn a_base_url_changed_on_a_profile_not_yet_taken_waits_too() {
    let stand = Stand::default();
    let made = SyncKey::generate().expect("key");
    let mut homes = [home("m_a", &made), home("m_b", &made)];
    set_profiles(&homes[0], glm(&homes[0]));
    rounds(&mut homes, &stand, 3).await;

    edit(&homes[0], |profile| {
        set_env(
            profile,
            "ANTHROPIC_BASE_URL",
            "https://elsewhere.example/api",
        )
    });
    rounds(&mut homes, &stand, 3).await;

    assert!(arrived(&homes[1], "01KGLMPROFILE"));
    assert_eq!(
        asked(&homes[1], "01KGLMPROFILE"),
        json!({"env.ANTHROPIC_BASE_URL": "https://elsewhere.example/api"})
    );
}
