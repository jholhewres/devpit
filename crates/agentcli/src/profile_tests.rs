use std::collections::HashSet;

use crate::profile::{found, offered, profiles, reach, Base, Declared, Reach};

fn declared(id: &str, command: &str) -> Declared {
    Declared {
        id: id.to_owned(),
        label: id.to_owned(),
        base: "claude".to_owned(),
        command: command.to_owned(),
        args: Vec::new(),
        models: Vec::new(),
        env: Vec::new(),
    }
}

/// The one base these tests use, standing in for the agent catalogue.
fn base(id: &str) -> Option<Base> {
    (id == "claude").then(|| Base {
        program: "claude".to_owned(),
        driver: "claude".to_owned(),
    })
}

/// Nothing the shell knows, which is what `profiles` was always told before
/// there was anything to tell it.
fn nothing() -> HashSet<String> {
    HashSet::new()
}

fn knows(names: &[&str]) -> HashSet<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn a_command_that_is_not_installed_is_still_listed() {
    // Dropping it would read as a profile that was never declared, and the
    // person would add it again and wonder why nothing changed.
    let all = profiles(
        &[declared("work", "definitely-not-a-real-command")],
        base,
        &nothing(),
    );
    let mine = all
        .iter()
        .find(|p| p.id == "work")
        .expect("the declared one");
    assert!(!mine.installed());
}

#[test]
fn a_declared_command_keeps_its_label() {
    let all = profiles(&[declared("personal", "sh")], base, &nothing());
    assert_eq!(all[0].label, "personal");
    assert!(all[0].installed(), "sh is on the PATH");
}

#[test]
fn two_profiles_can_name_two_commands() {
    // The whole point: one account per binary, both listed at once.
    let all = profiles(
        &[declared("work", "sh"), declared("personal", "ls")],
        base,
        &nothing(),
    );
    let declared_commands: Vec<_> = all
        .iter()
        .filter(|p| p.id == "work" || p.id == "personal")
        .map(|p| p.command.as_str())
        .collect();
    assert_eq!(declared_commands, ["sh", "ls"]);
}

#[test]
fn a_declared_command_is_not_listed_twice_by_discovery() {
    let all = profiles(&[declared("mine", "claude")], base, &knows(&["claude"]));
    assert_eq!(all.iter().filter(|p| p.command == "claude").count(), 1);
}

#[test]
fn a_command_that_is_not_executable_is_not_found() {
    assert!(found("definitely-not-a-real-command").is_none());
}

#[test]
fn a_file_on_the_path_is_runnable() {
    let (how, where_) = reach("sh", false);
    assert_eq!(how, Reach::Runnable);
    assert!(where_.is_some(), "a runnable command says where it is");
}

#[test]
fn a_name_only_the_shell_knows_is_shell_only() {
    // The case this exists for: `claudin` here is a function in `.zshrc` with
    // no file anywhere. The terminal runs it; `Command::new` cannot.
    let (how, where_) = reach("definitely-not-a-real-command", true);
    assert_eq!(how, Reach::ShellOnly);
    assert_eq!(where_, None, "there is no file to point at");
}

#[test]
fn a_name_nobody_knows_is_missing() {
    let (how, _) = reach("definitely-not-a-real-command", false);
    assert_eq!(how, Reach::Missing);
}

#[test]
fn a_file_wins_over_the_shell_knowing_the_name() {
    // Both true is the ordinary case for an installed CLI, and `Runnable` is
    // the answer that lets devpit spawn it rather than only type it.
    let (how, _) = reach("sh", true);
    assert_eq!(how, Reach::Runnable);
}

#[test]
fn a_shell_only_profile_is_installed_but_not_spawnable() {
    // The distinction the single boolean could not make.
    let all = profiles(
        &[declared("glm", "definitely-not-a-real-command")],
        base,
        &knows(&["definitely-not-a-real-command"]),
    );
    let mine = all.iter().find(|p| p.id == "glm").expect("declared");
    assert!(mine.installed(), "the terminal can start it");
    assert!(!mine.spawnable(), "devpit cannot");
}

#[test]
fn discovery_leaves_out_what_nothing_can_reach() {
    // A declared profile stays listed because somebody chose it; a discovered
    // one that is nowhere is noise.
    let all = profiles(&[], base, &nothing());
    assert!(
        all.iter().all(|p| p.installed()),
        "discovery listed something nothing can start"
    );
}

#[test]
fn a_profile_that_names_models_offers_those_after_the_accounts_default() {
    // `glm` runs against z.ai, which serves none of the driver's aliases.
    let mut glm = declared("glm", "sh");
    glm.models = vec!["glm-5.3[1m]".to_owned(), "glm-4.7".to_owned()];
    let all = profiles(&[glm, declared("plain", "sh")], base, &nothing());
    assert_eq!(all[0].models, ["default", "glm-5.3[1m]", "glm-4.7"]);
    // The editor reopens what was typed, not what the picker offers.
    assert_eq!(all[0].own_models, ["glm-5.3[1m]", "glm-4.7"]);
    assert_eq!(all[1].models, offered(&[], "claude"));
    assert!(all[1].models.len() > 1, "the driver's own list");
}

#[test]
fn a_default_the_person_placed_is_not_added_twice() {
    let own = ["glm-4.7".to_owned(), "default".to_owned()];
    assert_eq!(offered(&own, "claude"), ["glm-4.7", "default"]);
}

#[test]
fn another_account_of_the_same_cli_leaves_the_plain_one_listed() {
    // A gateway profile runs `claude` with variables of its own: it is not
    // the default sign-in, and must not hide it.
    let mut glm = declared("glm", "claude");
    glm.env = vec![devpit_rpc::EnvVar {
        name: "ANTHROPIC_BASE_URL".to_owned(),
        value: "https://example.test".to_owned(),
    }];
    let knows: HashSet<String> = ["claude".to_owned()].into();
    let listed = profiles(&[glm], base, &knows);
    assert!(
        listed.iter().any(|one| one.id == "claude" && !one.mine),
        "{listed:?}"
    );
}
