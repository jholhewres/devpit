use super::readable;

#[test]
fn a_shortcut_is_read_as_the_plugin_spells_it() {
    for keys in [
        "CommandOrControl+Shift+D",
        "Alt+Space",
        "Ctrl+Alt+P",
        "Super+D",
    ] {
        assert!(readable(keys), "{keys}");
    }
    for keys in ["", "Shift+", "Hyper+Banana", "D+D+D+"] {
        assert!(!readable(keys), "{keys}");
    }
}
