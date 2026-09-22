use super::*;

/// The 0.1.7 AppImage's list: GTK's own modules and nothing from the desktop.
const BUNDLED: &str = "\"im-cedilla.so\" \n\"cedilla\" \"Cedilla\" \"gtk30\" \"/usr/share/locale\" \"az:ca:co:fr:gv:oc:pt:sq:tr:wa\" \n";
const SYSTEM: &str = "\"/usr/lib/x86_64-linux-gnu/gtk-3.0/3.0.0/immodules/im-ibus.so\" \n\"ibus\" \"IBus (Intelligent Input Bus)\" \"ibus\" \"\" \"ja:ko:zh:*\" \n";

#[test]
fn the_desktop_names_its_input_method_either_way() {
    assert_eq!(named(Some("ibus"), None).as_deref(), Some("ibus"));
    assert_eq!(named(None, Some("@im=ibus")).as_deref(), Some("ibus"));
    assert_eq!(
        named(Some(" Fcitx "), Some("@im=ibus")).as_deref(),
        Some("fcitx")
    );
    assert_eq!(named(Some(""), Some("@im=none")), None);
    assert_eq!(named(None, None), None);
}

#[test]
fn a_list_offers_a_module_by_its_id() {
    assert!(offers(SYSTEM, "ibus"));
    assert!(!offers(BUNDLED, "ibus"));
    // A path that merely mentions the name is not the entry.
    assert!(!offers("\"/x/im-ibus.so\" \n", "ibus"));
}

#[test]
fn the_system_list_is_used_when_the_bundled_one_lacks_the_module() {
    let read = |path: &str| {
        if path == "/sys" {
            SYSTEM.to_owned()
        } else {
            String::new()
        }
    };
    assert_eq!(
        chosen(Some("ibus"), BUNDLED, &["/none", "/sys"], read),
        Some("/sys")
    );
}

#[test]
fn nothing_changes_when_the_bundled_list_has_it_or_nobody_asked() {
    let read = |_: &str| SYSTEM.to_owned();
    assert_eq!(chosen(Some("ibus"), SYSTEM, &["/sys"], read), None);
    assert_eq!(chosen(None, BUNDLED, &["/sys"], read), None);
    assert_eq!(chosen(Some("fcitx"), BUNDLED, &["/sys"], read), None);
}
