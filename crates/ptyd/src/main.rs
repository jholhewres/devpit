//! `devpit-ptyd`: listens at the socket [`devpit_ptyd::socket::path`] names
//! and serves devpit's terminals until it is killed. Started detached by the
//! desktop; one per user.

fn main() {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").map(std::path::PathBuf::from);
    let home = std::env::var_os("DEVPIT_HOME")
        .filter(|home| !home.is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| std::path::Path::new(&home).join(".devpit"))
        });
    let Some(home) = home else {
        eprintln!("devpit-ptyd: no HOME to find devpit's home in");
        std::process::exit(2);
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
