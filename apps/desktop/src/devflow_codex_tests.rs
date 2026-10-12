use super::*;

#[test]
fn codex_adds_devpits_folder_as_a_marketplace_before_installing_the_plugin_from_it() {
    let [market, install] = steps("/home/me/.devpit/devflow-plugin/codex");
    assert_eq!(
        market,
        [
            "plugin",
            "marketplace",
            "add",
            "/home/me/.devpit/devflow-plugin/codex"
        ]
    );
    assert_eq!(install, ["plugin", "add", "devflow@devpit"]);
}
