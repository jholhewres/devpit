//! Where the agent CLI keeps its configuration.
//!
//! `~/.claude` unless `CLAUDE_CONFIG_DIR` says otherwise, which is how a
//! person runs two installations of the same CLI side by side. Measured on one
//! machine: `~/.claude` held 54 skill directories and eight MCP servers, and
//! the directory the environment actually named held 24 and two.
//!
//! Reading the wrong one raises nothing anywhere. The panels simply describe
//! an installation that is not the one the agent will run — which is the exact
//! failure `mcp.rs` says it exists to catch, so it is worth one function that
//! three readers share rather than three spellings of a path.

use std::path::{Path, PathBuf};

use devpit_rpc::Credentials;

/// The configuration directory, given a home and what an environment says.
///
/// Takes the value rather than reading it so the rule can be tested without a
/// process environment, and so a caller holding a profile's own environment
/// can ask the same question about that profile.
pub fn config_dir_from(home: &Path, said: Option<&str>) -> PathBuf {
    match named(said) {
        Some(said) => PathBuf::from(said),
        None => home.join(".claude"),
    }
}

/// The configuration directory a process started with `env` would use.
///
/// The process's own environment carries over to the child, so what it
/// `inherited` counts when `env` says nothing; when `env` names the variable at
/// all, that is the value the child sees, empty included.
pub fn config_dir_of(home: &Path, env: &[(String, String)], inherited: Option<&str>) -> PathBuf {
    let own = env
        .iter()
        .rev()
        .find(|(name, _)| name == "CLAUDE_CONFIG_DIR")
        .map(|(_, value)| value.as_str());
    config_dir_from(home, own.or(inherited))
}

/// Whether `dir` holds a sign-in: `.credentials.json`, which is where the CLI
/// keeps it everywhere but macOS. Only asks that the file exists.
pub fn sign_in_at(dir: &Path) -> Credentials {
    if cfg!(target_os = "macos") {
        return Credentials::Unknown;
    }
    if dir.join(".credentials.json").is_file() {
        Credentials::Saved
    } else {
        Credentials::Missing
    }
}

/// The CLI's settings file, which is where its MCP servers are written.
///
/// Not simply inside the directory above. With nothing set, the CLI keeps
/// `~/.claude.json` *beside* `~/.claude`; only a named directory gets the file
/// inside it. Measured: `~/.claude/.claude.json` does not exist on a machine
/// whose `~/.claude.json` names eight servers, and joining the two paths the
/// obvious way made all eight disappear.
pub fn settings_file_from(home: &Path, said: Option<&str>) -> PathBuf {
    match named(said) {
        Some(said) => PathBuf::from(said).join(".claude.json"),
        None => home.join(".claude.json"),
    }
}

/// A setting that says something. An exported-but-empty variable is the
/// shape a half-written profile leaves behind, and reading it as a path lists
/// the filesystem root.
fn named(said: Option<&str>) -> Option<&str> {
    said.map(str::trim).filter(|said| !said.is_empty())
}

/// The configuration directory this process would use, or nothing when there
/// is no home to look in.
pub fn config_dir() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from)?;
    Some(config_dir_from(
        &home,
        std::env::var("CLAUDE_CONFIG_DIR").ok().as_deref(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_said_means_the_default_directory() {
        let home = Path::new("/home/someone");
        assert_eq!(
            config_dir_from(home, None),
            Path::new("/home/someone/.claude")
        );
    }

    #[test]
    fn what_the_environment_says_wins() {
        let home = Path::new("/home/someone");
        assert_eq!(
            config_dir_from(home, Some("/home/someone/.claude-other")),
            Path::new("/home/someone/.claude-other")
        );
    }

    #[test]
    fn the_default_settings_file_is_beside_the_directory_not_in_it() {
        let home = Path::new("/home/someone");
        assert_eq!(
            settings_file_from(home, None),
            Path::new("/home/someone/.claude.json")
        );
    }

    #[test]
    fn a_named_directory_holds_its_own_settings_file() {
        let home = Path::new("/home/someone");
        assert_eq!(
            settings_file_from(home, Some("/home/someone/.claude-other")),
            Path::new("/home/someone/.claude-other/.claude.json")
        );
    }

    #[test]
    fn a_profiles_own_directory_wins_over_the_inherited_one() {
        let home = Path::new("/home/someone");
        let env = [(
            "CLAUDE_CONFIG_DIR".to_owned(),
            "/home/someone/.claude-b".to_owned(),
        )];
        assert_eq!(
            config_dir_of(home, &env, Some("/home/someone/.claude-a")),
            Path::new("/home/someone/.claude-b")
        );
        assert_eq!(
            config_dir_of(home, &[], Some("/home/someone/.claude-a")),
            Path::new("/home/someone/.claude-a")
        );
    }

    #[test]
    fn a_directory_with_a_saved_sign_in_says_so() {
        let dir = tempfile::tempdir().expect("tempdir");
        if cfg!(target_os = "macos") {
            // The Keychain holds it there, so the file says nothing either way.
            assert_eq!(sign_in_at(dir.path()), Credentials::Unknown);
            return;
        }
        assert_eq!(sign_in_at(dir.path()), Credentials::Missing);
        std::fs::write(dir.path().join(".credentials.json"), "{}").expect("write");
        assert_eq!(sign_in_at(dir.path()), Credentials::Saved);
    }

    #[test]
    fn an_empty_setting_is_not_a_directory() {
        let home = Path::new("/home/someone");
        for said in ["", "   "] {
            assert_eq!(
                config_dir_from(home, Some(said)),
                Path::new("/home/someone/.claude")
            );
        }
    }
}
