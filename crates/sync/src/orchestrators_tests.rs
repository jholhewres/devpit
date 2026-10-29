use super::*;

#[test]
fn the_same_folder_name_mints_the_same_id_on_every_machine() {
    assert_eq!(minted("work"), minted("work"));
    assert_ne!(minted("work"), minted("home"));
    assert!(plain(&minted("work")));
}

#[test]
fn a_value_from_elsewhere_is_held_to_its_rules() {
    let hash = "0123456789abcdef0123456789abcdef";
    assert!(fits(&format!("{LINK}{hash}"), Some("on")));
    assert!(!fits(&format!("{LINK}{hash}"), Some("maybe")));
    assert!(!fits(&format!("{LINK}prj_01ABC"), Some("on")));
    assert!(!fits(&format!("{LINK}../x"), Some("on")));
    assert!(fits(PROFILE, Some("claude-work")));
    assert!(!fits(PROFILE, Some("/home/someone/.claude")));
    assert!(!fits(NAME, Some("  ")));
    assert!(!fits("folder", Some("/home/someone")));
}
