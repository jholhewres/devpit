use super::*;

use devpit_rpc::EnvVar;

const HOME: &str = "/home/me";

fn profile(id: &str, base: &str, config: Option<&str>) -> Declared {
    Declared {
        id: id.to_owned(),
        label: id.to_owned(),
        base: base.to_owned(),
        command: String::new(),
        args: Vec::new(),
        env: config
            .map(|dir| EnvVar {
                name: "CLAUDE_CONFIG_DIR".to_owned(),
                value: dir.to_owned(),
            })
            .into_iter()
            .collect(),
    }
}

fn directories(found: &[Found]) -> Vec<&str> {
    found
        .iter()
        .map(|one| one.directory.to_str().unwrap_or_default())
        .collect()
}

/// The machine this was written for: `claude` with nothing set, `claudin` and
/// `glm` each with a directory of their own.
fn this_machine() -> Vec<Declared> {
    vec![
        profile("claude-main", "claude", None),
        profile("claudin", "claude", Some("/home/me/.claude-claudin")),
        profile("glm", "claude", Some("/home/me/.claude-glm")),
    ]
}

#[test]
fn each_configuration_directory_is_one_installation() {
    let found = found_for(Path::new(HOME), None, &this_machine(), "", |_| true);
    assert_eq!(
        directories(&found),
        [
            "/home/me/.claude",
            "/home/me/.claude-claudin",
            "/home/me/.claude-glm"
        ]
    );
    // A profile with nothing set runs against the directory this process uses.
    assert_eq!(found[0].profiles, ["claude-main"]);
}

#[test]
fn two_profiles_on_one_directory_are_one_catalogue() {
    let mut declared = this_machine();
    declared.push(profile("glm-fast", "claude", Some("/home/me/.claude-glm")));
    let found = found_for(Path::new(HOME), None, &declared, "", |_| true);
    assert_eq!(found.len(), 3);
    assert_eq!(found[2].profiles, ["glm", "glm-fast"]);
}

#[test]
fn the_default_profile_decides_where_the_panels_start() {
    let found = found_for(Path::new(HOME), None, &this_machine(), "glm", |_| true);
    let picked = pick(found, None).expect("a default");
    assert_eq!(picked.directory, Path::new("/home/me/.claude-glm"));
}

#[test]
fn with_no_usable_default_the_panels_start_where_this_process_looks() {
    // An empty default is a plain shell; a Codex default has no skills here.
    for default_id in ["", "codex"] {
        let found = found_for(Path::new(HOME), None, &this_machine(), default_id, |_| true);
        let picked = pick(found, None).expect("a default");
        assert_eq!(picked.directory, Path::new("/home/me/.claude"));
    }
}

#[test]
fn another_clis_profile_is_not_an_installation_of_this_one() {
    let declared = vec![profile("codex-work", "codex", Some("/home/me/.codex-work"))];
    let found = found_for(Path::new(HOME), None, &declared, "", |_| true);
    assert_eq!(directories(&found), ["/home/me/.claude"]);
}

#[test]
fn a_directory_that_is_not_there_is_not_offered() {
    let found = found_for(Path::new(HOME), None, &this_machine(), "", |dir| {
        !dir.ends_with(".claude-glm")
    });
    assert!(!directories(&found).contains(&"/home/me/.claude-glm"));
}

#[test]
fn the_settings_file_follows_what_the_profile_said() {
    // Beside the default directory, inside a named one — kept on the found
    // installation so MCP can ask.
    let found = found_for(Path::new(HOME), None, &this_machine(), "", |_| true);
    assert_eq!(found[0].said, None);
    assert_eq!(found[1].said.as_deref(), Some("/home/me/.claude-claudin"));
}

#[test]
fn a_directory_the_screen_names_must_be_one_of_the_list() {
    // This process reads whatever it is pointed at, so a path from the window
    // is only accepted when it is one of these.
    let found = || found_for(Path::new(HOME), None, &this_machine(), "", |_| true);
    assert!(pick(found(), Some("/home/me/.claude-claudin")).is_ok());
    let refused = pick(found(), Some("/etc")).expect_err("a refusal");
    assert_eq!(refused.code, ErrorCode::Forbidden);
}
