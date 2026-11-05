use super::*;

/// The AppImage on a GNOME Wayland session, as its launcher leaves it.
fn appimage_on_wayland() -> Found<'static> {
    Found {
        appimage: true,
        wayland_display: true,
        asked: None,
        gdk_backend: Some("x11"),
        chosen: None,
        known_crash: false,
        failed_before: false,
    }
}

fn gdk(found: &Found) -> Option<&'static str> {
    decide(found).gdk_backend
}

#[test]
fn the_appimage_opens_on_wayland_over_its_launchers_x11() {
    let decision = decide(&appimage_on_wayland());
    assert_eq!(decision.gdk_backend, Some("wayland,x11"));
    assert_eq!(decision.why, DisplayWhy::Default);
}

#[test]
fn the_environment_override_wins_and_is_taken_exactly() {
    let asked = |value| Found {
        asked: Some(value),
        known_crash: true,
        failed_before: true,
        chosen: Some(DisplayBackend::X11),
        ..appimage_on_wayland()
    };
    assert_eq!(gdk(&asked("wayland")), Some("wayland"));
    assert_eq!(gdk(&asked(" X11 ")), Some("x11"));
    assert_eq!(decide(&asked("x11")).why, DisplayWhy::Asked);
    // Nonsense is not an override.
    assert_eq!(decide(&asked("mir")).why, DisplayWhy::KnownCrash);
}

#[test]
fn outside_the_appimage_a_backend_already_set_is_left_alone() {
    let found = Found {
        appimage: false,
        gdk_backend: Some("x11"),
        ..appimage_on_wayland()
    };
    assert_eq!(
        decide(&found),
        Decision {
            gdk_backend: None,
            why: DisplayWhy::Inherited
        }
    );
    let unset = Found {
        appimage: false,
        gdk_backend: None,
        ..appimage_on_wayland()
    };
    assert_eq!(gdk(&unset), Some("wayland,x11"));
}

#[test]
fn without_a_compositor_it_is_x11() {
    let found = Found {
        wayland_display: false,
        chosen: Some(DisplayBackend::Wayland),
        ..appimage_on_wayland()
    };
    assert_eq!(gdk(&found), Some("x11"));
    assert_eq!(decide(&found).why, DisplayWhy::NoWayland);
}

#[test]
fn the_known_crash_and_a_failed_start_fall_back_over_the_persons_choice() {
    let chosen = Some(DisplayBackend::Wayland);
    let crash = Found {
        known_crash: true,
        chosen,
        ..appimage_on_wayland()
    };
    assert_eq!(gdk(&crash), Some("x11"));
    assert_eq!(decide(&crash).why, DisplayWhy::KnownCrash);
    let failed = Found {
        failed_before: true,
        chosen,
        ..appimage_on_wayland()
    };
    assert_eq!(gdk(&failed), Some("x11"));
    assert_eq!(decide(&failed).why, DisplayWhy::FailedBefore);
}

#[test]
fn the_settings_choice_is_kept() {
    let chosen = |backend| Found {
        chosen: Some(backend),
        ..appimage_on_wayland()
    };
    assert_eq!(gdk(&chosen(DisplayBackend::X11)), Some("x11"));
    assert_eq!(gdk(&chosen(DisplayBackend::Wayland)), Some("wayland,x11"));
    assert_eq!(decide(&chosen(DisplayBackend::X11)).why, DisplayWhy::Chosen);
}

#[test]
fn a_wayland_start_that_never_loaded_its_page_fails_only_its_own_version() {
    let root = tempfile::tempdir().unwrap();
    assert!(!failed_before(root.path(), "0.1.47"));
    starting(root.path(), "0.1.47");
    assert!(failed_before(root.path(), "0.1.47"));
    // An update is a new try.
    assert!(!failed_before(root.path(), "0.1.48"));
    started(root.path());
    assert!(!failed_before(root.path(), "0.1.47"));
}

#[test]
fn a_stored_choice_reads_back_and_empty_is_automatic() {
    assert_eq!(parse("wayland"), Some(DisplayBackend::Wayland));
    assert_eq!(parse("X11"), Some(DisplayBackend::X11));
    assert_eq!(parse(""), None);
    for backend in [DisplayBackend::Wayland, DisplayBackend::X11] {
        assert_eq!(parse(spelled(backend)), Some(backend));
    }
}

#[test]
fn gtk_crashes_on_wayland_only_where_it_reads_a_missing_xsettings_key() {
    // GNOME 46: antialiasing moved to the deprecated schema, font-antialiasing in interface.
    let gnome_46 = (true, Some(false));
    // The bundled 3.24.33 reads interface's key: no crash.
    assert!(!crashes_on_wayland((3, 24, 33), gnome_46.0, gnome_46.1));
    // The Ubuntu 20.04 GTK of tauri#8541 reads the xsettings key it no longer has.
    assert!(crashes_on_wayland((3, 24, 20), gnome_46.0, gnome_46.1));
    // A newer GTK falls back to xsettings when interface lacks the key.
    assert!(crashes_on_wayland((3, 24, 33), false, Some(false)));
    // An old GNOME still has the key; a desktop without the schema is never read.
    assert!(!crashes_on_wayland((3, 24, 20), false, Some(true)));
    assert!(!crashes_on_wayland((3, 24, 20), false, None));
}
