//! The display GTK opens on Linux, decided before GTK starts.
//!
//! The AppImage's launcher exports `GDK_BACKEND=x11` (linuxdeploy's gtk hook,
//! for tauri#8541), which put every window through XWayland. That crash was a
//! GTK aborting on an xsettings schema without `antialiasing`, so it is checked
//! for here instead of assumed everywhere. Wayland is the default where there is
//! one, with X11 when GDK cannot reach the compositor (`wayland,x11`), when the
//! known crash would hit, or when the last Wayland start of this version never
//! loaded its page. `DEVPIT_GDK_BACKEND` beats all of it.

use std::path::Path;
use std::sync::OnceLock;

use devpit_core::{preference, Store};
use devpit_rpc::{Display, DisplayBackend, DisplayWhy, RpcError};

#[cfg(target_os = "linux")]
/// Wayland, and X11 when GDK cannot open the compositor's display.
const WAYLAND_FIRST: &str = "wayland,x11";

/// Left in devpit's folder while a Wayland start has not loaded its page yet.
const STARTING: &str = "display-start";

#[cfg(target_os = "linux")]
/// What this start was found with, before GTK started.
pub(crate) struct Found<'a> {
    pub appimage: bool,
    pub wayland_display: bool,
    /// `DEVPIT_GDK_BACKEND`.
    pub asked: Option<&'a str>,
    /// `GDK_BACKEND`, as found.
    pub gdk_backend: Option<&'a str>,
    pub chosen: Option<DisplayBackend>,
    pub known_crash: bool,
    pub failed_before: bool,
}

#[cfg(target_os = "linux")]
/// What `GDK_BACKEND` becomes (`None` leaves it as found), and why.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Decision {
    pub gdk_backend: Option<&'static str>,
    pub why: DisplayWhy,
}

pub(crate) fn parse(value: &str) -> Option<DisplayBackend> {
    match value.trim().to_ascii_lowercase().as_str() {
        "wayland" => Some(DisplayBackend::Wayland),
        "x11" => Some(DisplayBackend::X11),
        _ => None,
    }
}

fn spelled(backend: DisplayBackend) -> &'static str {
    match backend {
        DisplayBackend::Wayland => "wayland",
        DisplayBackend::X11 => "x11",
    }
}

#[cfg(target_os = "linux")]
pub(crate) fn decide(found: &Found) -> Decision {
    let to = |gdk_backend, why| Decision {
        gdk_backend: Some(gdk_backend),
        why,
    };
    // Exactly what was asked, with no fallback: it is how a backend is tried.
    if let Some(asked) = found.asked.and_then(parse) {
        return to(spelled(asked), DisplayWhy::Asked);
    }
    // Inside the AppImage the variable is the launcher's, never the person's.
    if !found.appimage && found.gdk_backend.is_some_and(|set| !set.is_empty()) {
        return Decision {
            gdk_backend: None,
            why: DisplayWhy::Inherited,
        };
    }
    if !found.wayland_display {
        return to("x11", DisplayWhy::NoWayland);
    }
    // Above the person's choice: a window that cannot open cannot be changed back.
    if found.known_crash {
        return to("x11", DisplayWhy::KnownCrash);
    }
    if found.failed_before {
        return to("x11", DisplayWhy::FailedBefore);
    }
    match found.chosen {
        Some(DisplayBackend::X11) => to("x11", DisplayWhy::Chosen),
        Some(DisplayBackend::Wayland) => to(WAYLAND_FIRST, DisplayWhy::Chosen),
        None => to(WAYLAND_FIRST, DisplayWhy::Default),
    }
}

#[cfg(target_os = "linux")]
/// Whether a Wayland start of `version` was left without its page loaded.
pub(crate) fn failed_before(root: &Path, version: &str) -> bool {
    std::fs::read_to_string(root.join(STARTING)).is_ok_and(|left| left.trim() == version)
}

#[cfg(target_os = "linux")]
pub(crate) fn starting(root: &Path, version: &str) {
    let _ = std::fs::write(root.join(STARTING), version);
}

pub(crate) fn started(root: &Path) {
    let _ = std::fs::remove_file(root.join(STARTING));
}

static WHY: OnceLock<DisplayWhy> = OnceLock::new();
static RUNNING: OnceLock<DisplayBackend> = OnceLock::new();

fn chosen(store: &Store) -> Option<DisplayBackend> {
    store
        .preference(preference::DISPLAY_BACKEND)
        .ok()
        .flatten()
        .as_deref()
        .and_then(parse)
}

/// Whether GTK 3 on Wayland would abort reading font settings (tauri#8541).
///
/// It reads `antialiasing` from the xsettings schema, which GNOME 40 moved out;
/// from 3.24.23 only when `org.gnome.desktop.interface` lacks `font-antialiasing`.
/// `xsettings_has_key` is `None` where that schema is not installed at all.
#[cfg(target_os = "linux")]
pub(crate) fn crashes_on_wayland(
    gtk: (u32, u32, u32),
    interface_has_key: bool,
    xsettings_has_key: Option<bool>,
) -> bool {
    let reads_xsettings = gtk < (3, 24, 23) || !interface_has_key;
    reads_xsettings && xsettings_has_key == Some(false)
}

#[cfg(target_os = "linux")]
fn known_crash() -> bool {
    let Some(source) = gtk::gio::SettingsSchemaSource::default() else {
        return false;
    };
    let has = |schema: &str, key: &str| source.lookup(schema, true).map(|found| found.has_key(key));
    let gtk = (
        gtk::major_version(),
        gtk::minor_version(),
        gtk::micro_version(),
    );
    crashes_on_wayland(
        gtk,
        has("org.gnome.desktop.interface", "font-antialiasing").unwrap_or(false),
        has(
            "org.gnome.settings-daemon.plugins.xsettings",
            "antialiasing",
        ),
    )
}

/// Sets `GDK_BACKEND` for this start. Before GTK and before any thread.
#[cfg(target_os = "linux")]
pub(crate) fn choose() {
    let var = |name| std::env::var(name).ok();
    let (asked, gdk_backend) = (var("DEVPIT_GDK_BACKEND"), var("GDK_BACKEND"));
    let wayland_display = std::env::var_os("WAYLAND_DISPLAY").is_some_and(|set| !set.is_empty());
    let root = Store::root().ok();
    let version = env!("CARGO_PKG_VERSION");
    let found = Found {
        appimage: std::env::var_os("APPDIR").is_some(),
        wayland_display,
        asked: asked.as_deref(),
        gdk_backend: gdk_backend.as_deref(),
        chosen: Store::open_default().ok().as_ref().and_then(chosen),
        known_crash: wayland_display && known_crash(),
        failed_before: root
            .as_deref()
            .is_some_and(|root| failed_before(root, version)),
    };
    let decision = decide(&found);
    if let Some(value) = decision.gdk_backend {
        // Single-threaded still: nothing has been spawned before this.
        std::env::set_var("GDK_BACKEND", value);
        if let (true, Some(root)) = (value.starts_with("wayland"), root.as_deref()) {
            starting(root, version);
        }
    }
    if matches!(
        decision.why,
        DisplayWhy::KnownCrash | DisplayWhy::FailedBefore
    ) {
        eprintln!("devpit: on X11 instead of Wayland ({:?})", decision.why);
    }
    let _ = WHY.set(decision.why);
}

#[cfg(target_os = "linux")]
/// The main window's page loaded: this start's display works.
pub(crate) fn page_loaded(wayland: bool) {
    let _ = RUNNING.set(if wayland {
        DisplayBackend::Wayland
    } else {
        DisplayBackend::X11
    });
    if let Ok(root) = Store::root() {
        started(&root);
    }
}

fn now(store: &Store) -> Display {
    Display {
        offered: cfg!(target_os = "linux"),
        running: RUNNING.get().copied(),
        chosen: chosen(store),
        why: WHY.get().copied().unwrap_or(DisplayWhy::Default),
    }
}

/// `display.read` — what this start runs on, why, and the choice for the next.
#[tauri::command]
#[specta::specta]
pub async fn display_read() -> Result<Display, RpcError> {
    crate::off_main::blocking(|| Ok(now(&Store::open_default()?))).await
}

/// `display.set` — the display for the next start; null is automatic.
#[tauri::command]
#[specta::specta]
pub async fn display_set(backend: Option<DisplayBackend>) -> Result<Display, RpcError> {
    crate::off_main::blocking(move || {
        let store = Store::open_default()?;
        store.set_preference(preference::DISPLAY_BACKEND, backend.map_or("", spelled))?;
        // A choice made is a new try: a failed Wayland start no longer holds it.
        started(&Store::root()?);
        Ok(now(&store))
    })
    .await
}

#[cfg(all(test, target_os = "linux"))]
#[path = "display_backend_tests.rs"]
mod tests;
