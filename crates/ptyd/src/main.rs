//! `devpit-ptyd`: listens at the socket [`devpit_ptyd::socket::path`] names
//! and serves devpit's terminals until it is killed. Started detached by the
//! desktop; one per user and build profile.

#[cfg(unix)]
fn main() {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").map(std::path::PathBuf::from);
    // devpit's own rule for its home: DEVPIT_HOME, else the profile's root.
    let home = match devpit_core::Store::root() {
        Ok(home) => home,
        Err(err) => {
            eprintln!("devpit-ptyd: no home for devpit: {err}");
            std::process::exit(2);
        }
    };
    let path = std::env::args_os()
        .nth(1)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| devpit_ptyd::socket::path(runtime.as_deref(), &home));
    let listener = match devpit_ptyd::socket::listen(&path) {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!("devpit-ptyd: cannot listen at {}: {err}", path.display());
            std::process::exit(1);
        }
    };
    devpit_ptyd::daemon::Daemon::new().serve(listener);
}

/// Terminals there stay on psmux; the daemon is unix sockets and ptys.
#[cfg(not(unix))]
fn main() {
    eprintln!("devpit-ptyd runs on Linux and macOS only");
    std::process::exit(2);
}
