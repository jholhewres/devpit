//! On Windows, the tmux devpit's terminals run on is psmux, installed beside
//! devpit in `bin/`. It is put first on `PATH` once, at start, so every
//! `tmux` this process and its children run is that one.

#[cfg(windows)]
pub(crate) fn use_the_bundled_tmux() {
    let Some(bin) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("bin")))
        .filter(|bin| bin.join("tmux.exe").is_file())
    else {
        return;
    };
    let own = std::env::var_os("PATH").unwrap_or_default();
    let joined = std::env::join_paths(std::iter::once(bin).chain(std::env::split_paths(&own)));
    // At the top of `main`, before any other thread exists to read it.
    if let Ok(path) = joined {
        std::env::set_var("PATH", path);
    }
}

#[cfg(not(windows))]
pub(crate) fn use_the_bundled_tmux() {}
