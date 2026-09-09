use crate::profile::{found, profiles, Profile};

fn declared(id: &str, command: &str) -> Profile {
    Profile {
        id: id.to_owned(),
        label: id.to_owned(),
        command: command.to_owned(),
        driver: "claude".to_owned(),
        path: None,
        models: Vec::new(),
        efforts: Vec::new(),
        effort_default: None,
    }
}

#[test]
fn a_command_that_is_not_installed_is_still_listed() {
    // Dropping it would read as a profile that was never declared, and the
    // person would add it again and wonder why nothing changed.
    let all = profiles(&[declared("work", "definitely-not-a-real-command")]);
    let mine = all
        .iter()
        .find(|p| p.id == "work")
        .expect("the declared one");
    assert!(!mine.installed());
}

#[test]
fn a_declared_command_keeps_its_label() {
    let all = profiles(&[declared("personal", "sh")]);
    assert_eq!(all[0].label, "personal");
    assert!(all[0].installed(), "sh is on the PATH");
}

#[test]
fn two_profiles_can_name_two_commands() {
    // The whole point: one account per binary, both listed at once.
    let all = profiles(&[declared("work", "sh"), declared("personal", "ls")]);
    let declared_commands: Vec<_> = all
        .iter()
        .filter(|p| p.id == "work" || p.id == "personal")
        .map(|p| p.command.as_str())
        .collect();
    assert_eq!(declared_commands, ["sh", "ls"]);
}

#[test]
fn a_declared_command_is_not_listed_twice_by_discovery() {
    let all = profiles(&[declared("mine", "claude")]);
    assert_eq!(all.iter().filter(|p| p.command == "claude").count(), 1);
}

#[test]
fn a_command_that_is_not_executable_is_not_found() {
    assert!(found("definitely-not-a-real-command").is_none());
}
