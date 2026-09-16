use super::*;

/// Both halves of the answer: what the plugins installed, and what the person
/// wrote themselves.
#[test]
fn a_plugins_skills_and_the_persons_own_are_both_sources() {
    let dir = tempfile::tempdir().expect("tempdir");
    let home = dir.path().join("home");
    let cli = home.join(".claude");
    let plugin = cli.join("plugins/cache/omc/pack/skills");
    std::fs::create_dir_all(&plugin).expect("create");
    std::fs::create_dir_all(cli.join("skills")).expect("create");
    std::fs::create_dir_all(home.join(".devpit/skills")).expect("create");

    let found = sources_in(&cli, &home);

    assert!(found.contains(&plugin), "the plugin's skills are not there");
    assert!(found.contains(&cli.join("skills")));
    assert!(found.contains(&home.join(".devpit/skills")));
}

/// A directory that is not there is not an empty set of skills — it is one
/// fewer place to look, and saying so would be a different claim.
#[test]
fn a_directory_that_does_not_exist_is_not_a_source() {
    let dir = tempfile::tempdir().expect("tempdir");
    let home = dir.path().join("home");
    assert!(sources_in(&home.join(".claude"), &home).is_empty());
}
