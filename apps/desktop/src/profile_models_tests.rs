use devpit_rpc::Declared;

use super::*;

fn open(dir: &Path) -> Store {
    Store::open(&dir.join("state.db")).expect("open")
}

fn kept_as(store: &Store, profile_id: &str, models: ProfileModels) {
    let mut all = kept(store);
    all.insert(profile_id.to_owned(), models);
    store
        .set_preference(
            preference::AGENTS_PROFILE_MODELS,
            &serde_json::to_string(&all).expect("json"),
        )
        .expect("write");
}

#[test]
fn every_model_is_on_and_there_is_no_advisor_until_one_is_set() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert_eq!(of(&open(dir.path()), "glm"), ProfileModels::default());
}

#[test]
fn a_model_switched_off_leaves_the_profiles_offered_models() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = open(dir.path());
    let declared = Declared {
        id: "glm".to_owned(),
        label: "GLM".to_owned(),
        base: "claude".to_owned(),
        command: String::new(),
        args: Vec::new(),
        env: Vec::new(),
        models: vec!["glm-4.6".to_owned(), "glm-4.5-air".to_owned()],
    };
    crate::agent_profiles::save(&store, &[declared]).expect("save");
    let offered = |store: &Store| {
        crate::agent_profiles::all(store)
            .expect("profiles")
            .into_iter()
            .find(|one| one.id == "glm")
            .expect("glm")
            .models
    };
    assert!(offered(&store).contains(&"glm-4.5-air".to_owned()));

    kept_as(
        &store,
        "glm",
        ProfileModels {
            models_off: vec!["glm-4.5-air".to_owned()],
            advisor_model: None,
        },
    );
    let now = offered(&store);
    assert!(!now.contains(&"glm-4.5-air".to_owned()), "{now:?}");
    assert!(now.contains(&"glm-4.6".to_owned()), "{now:?}");
}

#[test]
fn only_model_names_are_kept_and_an_empty_advisor_is_none() {
    assert!(checked(ProfileModels {
        models_off: vec!["opus; rm -rf".to_owned()],
        advisor_model: None,
    })
    .is_err());
    assert!(checked(ProfileModels {
        models_off: Vec::new(),
        advisor_model: Some("not a model".to_owned()),
    })
    .is_err());
    assert_eq!(
        checked(ProfileModels {
            models_off: vec!["haiku".to_owned()],
            advisor_model: Some("  ".to_owned()),
        }),
        Ok(ProfileModels {
            models_off: vec!["haiku".to_owned()],
            advisor_model: None,
        })
    );
}

#[test]
fn the_default_model_is_never_switched_off() {
    assert!(checked(ProfileModels {
        models_off: vec![DEFAULT_MODEL.to_owned()],
        advisor_model: None,
    })
    .is_err());
}

#[test]
fn a_profile_with_an_advisor_gets_a_hooks_file_of_its_own() {
    let root = Path::new("/home/me/.devpit");
    assert_eq!(hooks_file(root, None), root.join("hooks.json"));
    assert_eq!(hooks_file(root, Some("glm")), root.join("hooks-glm.json"));
    assert_eq!(
        hooks_file(root, Some("../x y")),
        root.join("hooks-___x_y.json")
    );
}

#[test]
fn the_advisor_is_written_in_devpits_settings_for_the_profiles_sessions_only() {
    let profile = format!("advisor-{}", ulid::Ulid::generate());
    let store = Store::open_default().expect("store");
    kept_as(
        &store,
        &profile,
        ProfileModels {
            models_off: Vec::new(),
            advisor_model: Some("opus".to_owned()),
        },
    );
    let own = crate::steps::hook_settings_for(Some(&profile)).expect("own settings");
    assert!(own.ends_with(&format!("hooks-{profile}.json")), "{own}");
    let written: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&own).expect("read")).expect("json");
    assert_eq!(written["advisorModel"], "opus");
    assert!(written["hooks"].is_object());

    let shared = crate::steps::hook_settings_for(Some("no-advisor-here")).expect("shared");
    assert!(shared.ends_with("hooks.json"), "{shared}");
    let plain: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&shared).expect("read")).expect("json");
    assert!(plain.get("advisorModel").is_none());
}

#[test]
fn checks_run_every_five_minutes_until_another_offered_time_is_chosen() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = open(dir.path());
    assert_eq!(check_minutes(&store), 5);
    store
        .set_preference(preference::PROVIDERS_CHECK_MINUTES, "15")
        .expect("write");
    assert_eq!(check_minutes(&store), 15);
    store
        .set_preference(preference::PROVIDERS_CHECK_MINUTES, "2")
        .expect("write");
    assert_eq!(check_minutes(&store), 5);
}
