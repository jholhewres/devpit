//! The ssh wrapper, run through `sh` with a fake `ssh` that writes down what
//! it was asked to do.
//!
//! A real login was checked by hand against a local sshd, from bash and from
//! zsh, inside tmux and out. What is pinned here is the part that decides:
//! which arguments are a login and which are plain ssh, and that the line
//! handed to the remote side starts the remote shell through devpit's own
//! startup file and leaves nothing behind.

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

struct Rig {
    dir: tempfile::TempDir,
    root: PathBuf,
}

/// The installed wrappers, and a `bin` holding a fake `ssh`. The fake says
/// `remotecommand` for `-G` when the arguments ask for one, as the real one
/// does, and otherwise writes its arguments one per line.
fn rig() -> Option<Rig> {
    let tools = Command::new("sh")
        .args([
            "-c",
            "command -v gzip && command -v base64 && command -v mktemp",
        ])
        .output()
        .ok()?;
    if !tools.status.success() {
        return None;
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("wrappers");
    devpit_pty::shell::install(&root).expect("install");
    let bin = dir.path().join("bin");
    std::fs::create_dir(&bin).unwrap();
    write_script(
        &bin.join("ssh"),
        &format!(
            "#!/bin/sh\n\
             if [ \"$1\" = -G ]; then\n\
               case \"$*\" in *RemoteCommand*) echo 'remotecommand uptime' ;; esac\n\
               echo 'user someone'\n\
               exit 0\n\
             fi\n\
             printf '%s\\n' \"$@\" > '{}'\n",
            dir.path().join("asked").display()
        ),
    );
    Some(Rig { dir, root })
}

fn write_script(path: &Path, body: &str) {
    std::fs::write(path, body).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

impl Rig {
    /// What the fake ssh was run with, for these arguments to the wrapper.
    fn asked(&self, args: &[&str]) -> Vec<String> {
        let _ = std::fs::remove_file(self.dir.path().join("asked"));
        let path = format!(
            "{}:{}",
            self.dir.path().join("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let status = Command::new("sh")
            .arg(self.root.join("ssh").join("login.sh"))
            .arg(&self.root)
            .args(args)
            .env("PATH", path)
            .env_remove("TMUX")
            .status()
            .expect("run the wrapper");
        assert!(status.success());
        std::fs::read_to_string(self.dir.path().join("asked"))
            .expect("ssh was run")
            .lines()
            .map(str::to_owned)
            .collect()
    }
}

#[test]
fn only_a_plain_login_is_wrapped() {
    let Some(rig) = rig() else { return };
    for login in [
        &["host"][..],
        &["-p", "2222", "host"],
        &["-p2222", "-v", "host"],
        &["host", "-v"],
        &["-l", "me", "--", "host"],
    ] {
        let asked = rig.asked(login);
        assert_eq!(asked[0], "-t", "{login:?} is a login: {asked:?}");
        assert!(
            asked.last().unwrap().starts_with("exec sh -c '"),
            "{login:?}"
        );
    }
    for plain in [
        &["host", "uptime"][..],
        &["-N", "-L", "8080:localhost:80", "host"],
        &["-T", "host"],
        &["-vT", "host"],
        &["-W", "db:5432", "jump"],
        &["-s", "host", "sftp"],
        &["-o", "RemoteCommand=uptime", "host"],
        &["--", "host", "uptime"],
    ] {
        assert_eq!(rig.asked(plain), plain, "{plain:?} is left alone");
    }
}

#[test]
fn the_remote_line_starts_the_login_shell_through_devpits_startup_file() {
    let Some(rig) = rig() else { return };
    let remote = rig.asked(&["host"]).pop().unwrap();
    // Read the same way by sh, bash, zsh, fish and csh on the far side.
    let inner = &remote["exec sh -c '".len()..remote.len() - 1];
    for byte in ['\'', '\\', '!', '\n'] {
        assert!(!inner.contains(byte), "{byte:?} in the remote line");
    }

    // The remote side, played here: a login shell named bash that says how
    // it was started, and what the file it was handed says.
    let far = rig.dir.path().join("far");
    std::fs::create_dir(&far).unwrap();
    let said = rig.dir.path().join("said");
    write_script(
        &far.join("bash"),
        &format!(
            "#!/bin/sh\n\
             {{ echo \"$1\"; echo \"$DEVPIT_SHELL_FEATURES\"; head -1 \"$2\"; \
             grep -c __devpit_precmd \"$2\"; }} > '{}'\n",
            said.display()
        ),
    );
    let tmp = rig.dir.path().join("tmp");
    std::fs::create_dir(&tmp).unwrap();
    let status = Command::new("sh")
        .args(["-c", &remote])
        .env("SHELL", far.join("bash"))
        .env("TMPDIR", &tmp)
        .env_remove("DEVPIT_SHELL_FEATURES")
        .status()
        .expect("run the remote line");
    assert!(status.success());
    let said = std::fs::read_to_string(said).expect("the shell was started");
    let lines: Vec<&str> = said.lines().collect();
    assert_eq!(lines[0], "--rcfile");
    assert_eq!(lines[1], "marks");
    assert!(
        lines[2].starts_with("command rm -rf -- '"),
        "the file removes its own folder first: {:?}",
        lines[2]
    );
    assert_ne!(lines[3], "0", "it is devpit's bash startup file");
}

#[test]
fn a_remote_shell_with_no_startup_file_here_is_a_plain_login() {
    let Some(rig) = rig() else { return };
    let remote = rig.asked(&["host"]).pop().unwrap();
    let far = rig.dir.path().join("far");
    std::fs::create_dir(&far).unwrap();
    let said = rig.dir.path().join("said");
    write_script(
        &far.join("fish"),
        &format!("#!/bin/sh\necho \"$@\" > '{}'\n", said.display()),
    );
    let tmp = rig.dir.path().join("tmp");
    std::fs::create_dir(&tmp).unwrap();
    let status = Command::new("sh")
        .args(["-c", &remote])
        .env("SHELL", far.join("fish"))
        .env("TMPDIR", &tmp)
        .status()
        .expect("run the remote line");
    assert!(status.success());
    assert_eq!(std::fs::read_to_string(said).unwrap().trim(), "-l");
    assert_eq!(
        std::fs::read_dir(&tmp).unwrap().count(),
        0,
        "nothing of devpit is left on the other machine"
    );
}
