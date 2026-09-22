//! The input method an AppImage leaves out.
//!
//! The AppImage points GTK at its own list of input-method modules, and that
//! list has no IBus and no Fcitx: those belong to the desktop, not to the app.
//! A desktop typing through IBus then reaches devpit's GTK with no module to
//! talk to it, GTK composes dead keys on its own while IBus composes them too,
//! and `´` then `i` arrives as `B B     í í` — letters doubled, a stray
//! capital, a run of spaces. Recorded twice from ABNT2 keyboards on 0.1.7.
//!
//! The fix is to hand GTK the system's list when the bundled one lacks the
//! module the desktop names. It is read before GTK starts, which is why this
//! runs first thing in `main` — the AppImage's launcher sets the variable
//! itself, so it cannot be overridden from outside.

use std::path::Path;

/// Where distributions keep GTK 3's module list.
const SYSTEM_CACHES: [&str; 4] = [
    "/usr/lib/x86_64-linux-gnu/gtk-3.0/3.0.0/immodules.cache",
    "/usr/lib/aarch64-linux-gnu/gtk-3.0/3.0.0/immodules.cache",
    "/usr/lib64/gtk-3.0/3.0.0/immodules.cache",
    "/usr/lib/gtk-3.0/3.0.0/immodules.cache",
];

/// Points GTK at the system's module list when running from an AppImage whose
/// own list lacks the desktop's input method. Does nothing anywhere else.
pub(crate) fn use_the_desktops() {
    if std::env::var_os("APPDIR").is_none() {
        return;
    }
    let Ok(bundled) = std::env::var("GTK_IM_MODULE_FILE") else {
        return;
    };
    let named = named(
        std::env::var("GTK_IM_MODULE").ok().as_deref(),
        std::env::var("XMODIFIERS").ok().as_deref(),
    );
    let read = |path: &str| std::fs::read_to_string(Path::new(path)).unwrap_or_default();
    if let Some(system) = chosen(named.as_deref(), &read(&bundled), &SYSTEM_CACHES, read) {
        // Single-threaded still: nothing has been spawned before this.
        std::env::set_var("GTK_IM_MODULE_FILE", system);
    }
}

/// The input method the desktop asked for: `GTK_IM_MODULE` when it is set,
/// otherwise the one `XMODIFIERS` names (`@im=ibus`).
pub(crate) fn named(gtk_im_module: Option<&str>, xmodifiers: Option<&str>) -> Option<String> {
    let direct = gtk_im_module.map(str::trim).filter(|name| !name.is_empty());
    let from_x = xmodifiers
        .and_then(|value| value.trim().strip_prefix("@im="))
        .map(str::trim)
        .filter(|name| !name.is_empty() && *name != "none");
    direct.or(from_x).map(str::to_lowercase)
}

/// Whether a module list offers `name`. Each module's entry starts with its id
/// in quotes: `"ibus" "IBus (Intelligent Input Bus)" …`.
pub(crate) fn offers(cache: &str, name: &str) -> bool {
    let id = format!("\"{name}\" ");
    cache.lines().any(|line| line.starts_with(&id))
}

/// The system list to use instead, if the bundled one lacks the module and a
/// system one has it.
pub(crate) fn chosen<'a>(
    named: Option<&str>,
    bundled: &str,
    systems: &[&'a str],
    read: impl Fn(&str) -> String,
) -> Option<&'a str> {
    let name = named?;
    if offers(bundled, name) {
        return None;
    }
    systems
        .iter()
        .copied()
        .find(|path| offers(&read(path), name))
}

#[cfg(test)]
#[path = "input_method_tests.rs"]
mod tests;
