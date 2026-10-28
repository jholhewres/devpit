use devpit_rpc::EnvVar;

use super::*;

fn profile(id: &str, config_dir: Option<&str>) -> Declared {
    Declared {
        id: id.to_owned(),
        label: id.to_owned(),
        base: "claude".to_owned(),
        command: String::new(),
        args: Vec::new(),
        env: config_dir
            .map(|dir| EnvVar {
                name: "CLAUDE_CONFIG_DIR".to_owned(),
                value: dir.to_owned(),
            })
            .into_iter()
            .collect(),
        models: Vec::new(),
    }
}

fn declared() -> Vec<Declared> {
    vec![
        profile("company", None),
        profile("claudin", Some("/home/p/.claude-claudin")),
        profile("glm", Some("/home/p/.claude-glm")),
    ]
}

fn id(chosen: Option<&Declared>) -> Option<&str> {
    chosen.map(|one| one.id.as_str())
}

#[test]
fn the_profile_the_project_names_comes_first() {
    let all = declared();
    let asked = Asked {
        named: Some("glm"),
        asker_config: Some(Path::new("/home/p/.claude-claudin")),
        linked: &["claudin".to_owned()],
    };
    assert_eq!(id(chosen(&all, &asked)), Some("glm"));
}

#[test]
fn without_one_the_asking_sessions_account_decides() {
    let all = declared();
    let asked = Asked {
        named: None,
        asker_config: Some(Path::new("/home/p/.claude-claudin/")),
        linked: &["company".to_owned()],
    };
    assert_eq!(id(chosen(&all, &asked)), Some("claudin"));
}

#[test]
fn then_the_orchestrators_linked_to_the_project_when_they_agree() {
    let all = declared();
    let one = Asked {
        named: None,
        asker_config: None,
        linked: &["claudin".to_owned(), "claudin".to_owned()],
    };
    assert_eq!(id(chosen(&all, &one)), Some("claudin"));
    let two = Asked {
        linked: &["claudin".to_owned(), "company".to_owned()],
        ..one
    };
    assert_eq!(id(chosen(&all, &two)), None);
}

#[test]
fn nothing_said_keeps_the_machines_default() {
    let all = declared();
    let asked = Asked {
        named: None,
        asker_config: Some(Path::new("/elsewhere")),
        linked: &[],
    };
    assert_eq!(id(chosen(&all, &asked)), None);
    // A profile that is no longer declared names nothing.
    let gone = Asked {
        named: Some("deleted"),
        ..asked
    };
    assert_eq!(id(chosen(&all, &gone)), None);
}

#[test]
fn a_claude_profile_gives_the_reviewers_its_environment_and_another_does_not() {
    let claudin = profile("claudin", Some("/home/p/.claude-claudin"));
    assert_eq!(
        env_of(Some(&claudin)),
        [(
            "CLAUDE_CONFIG_DIR".to_owned(),
            "/home/p/.claude-claudin".to_owned()
        )]
    );
    let codex = Declared {
        base: "codex".to_owned(),
        ..claudin
    };
    assert!(env_of(Some(&codex)).is_empty());
    assert!(env_of(None).is_empty());
}

#[test]
fn the_sign_in_is_read_where_the_profile_keeps_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = dir.path().join("claudin");
    std::fs::create_dir_all(&config).expect("config");
    let claudin = profile("claudin", Some(&config.display().to_string()));
    let inherited = Some("/home/p/.claude-company");
    assert_eq!(config_dir(Some(&claudin), dir.path(), inherited), config);
    assert_eq!(
        config_dir(None, dir.path(), inherited),
        Path::new("/home/p/.claude-company"),
        "no profile is the account this process runs on"
    );
    assert_eq!(
        config_dir(None, dir.path(), None),
        dir.path().join(".claude")
    );
}

#[test]
fn a_projects_profile_is_kept_and_cleared() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("store");
    assert_eq!(named(&store, "prj_a"), None);
    set_named(&store, "prj_a", Some("claudin")).expect("kept");
    assert_eq!(named(&store, "prj_a").as_deref(), Some("claudin"));
    assert_eq!(named(&store, "prj_b"), None);
    set_named(&store, "prj_a", None).expect("cleared");
    assert_eq!(named(&store, "prj_a"), None);
}

fn listed(declared: &[Declared], shell_functions: &[&str]) -> Vec<Profile> {
    let base = |id: &str| {
        Some(devpit_agentcli::profile::Base {
            program: id.to_owned(),
            driver: id.to_owned(),
        })
    };
    let knows = shell_functions
        .iter()
        .map(|one| (*one).to_owned())
        .collect();
    devpit_agentcli::profile::profiles(declared, base, &knows)
}

#[test]
fn a_reviewer_starts_a_profiles_program_only_when_devpit_can_spawn_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    let program = dir.path().join("claude-work");
    std::fs::write(&program, "#!/bin/sh\n").expect("program");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755))
            .expect("executable");
    }
    let runnable = Declared {
        command: program.display().to_string(),
        ..profile("work", None)
    };
    let function = Declared {
        command: "claude2".to_owned(),
        ..profile("fn", None)
    };
    let codex = Declared {
        base: "codex".to_owned(),
        ..runnable.clone()
    };
    let all = listed(&[runnable, function, codex], &["claude2"]);
    assert_eq!(
        program_from(&all[0]).as_deref(),
        Some(program.to_str().expect("utf-8"))
    );
    assert_eq!(
        program_from(&all[1]),
        None,
        "a shell function is not spawned"
    );
    assert_eq!(program_from(&all[2]), None, "not a Claude profile");
}

#[test]
fn the_screen_says_why_and_forgets_a_deleted_profile() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("store");
    crate::agent_profiles::save(&store, &declared()).expect("profiles");

    let none = shown(&store, "prj_a");
    assert_eq!(
        (none.named, none.runs_as, none.from),
        (None, None, DevflowProfileFrom::Default)
    );
    assert_eq!(none.choices.len(), 3);

    set_named(&store, "prj_a", Some("glm")).expect("named");
    let named = shown(&store, "prj_a");
    assert_eq!(named.named.as_deref(), Some("glm"));
    assert_eq!(named.from, DevflowProfileFrom::Project);

    set_named(&store, "prj_a", Some("deleted")).expect("named");
    let gone = shown(&store, "prj_a");
    assert_eq!((gone.named, gone.from), (None, DevflowProfileFrom::Default));
}

#[test]
fn a_named_profile_since_deleted_falls_through_to_the_asking_session() {
    let all = declared();
    let asked = Asked {
        named: Some("deleted"),
        asker_config: Some(Path::new("/home/p/.claude-claudin")),
        linked: &[],
    };
    assert_eq!(id(chosen(&all, &asked)), Some("claudin"));
}
