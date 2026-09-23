//! How a shell is started so that [`crate::osc`] has something to read.
//!
//! A shell says nothing about itself by default. `zsh` and `bash` draw a
//! prompt, run a command and print its output, and from outside those are one
//! undifferentiated stream of bytes — there is no way to tell where the prompt
//! ended, what the person typed, or whether it worked. The scanner next door
//! can read all three, and reads nothing, because nothing is emitting them.
//!
//! So the shell is launched with a startup file of ours that installs the
//! hooks: `bash` through `--rcfile`, `zsh` through `ZDOTDIR`. Each emits the
//! OSC 133 sequences the scanner already understands, plus two private ones
//! that say the shell is up and which process it is.
//!
//! Three rules, all learnt from Orca's wrapper, all of which cost them a
//! defect first:
//!
//! 1. **Chain, never replace.** The person's own `precmd_functions`, DEBUG
//!    trap and `zle-line-init` keep working; a wrapper that overwrites them
//!    breaks starship, bash-preexec and every prompt framework at once.
//! 2. **Re-apply after their config.** The startup file runs in the middle,
//!    and it can undo the environment we set at spawn.
//! 3. **Ask for features positively, and destroy the request.** The variable
//!    naming what to turn on is consumed in the first lines, so nothing the
//!    shell later spawns can inherit it — including another copy of this app
//!    started from that very terminal.
//!
//! What a rule can and cannot prove: the tests below check the launch and the
//! shape of the files, because those are rules. That the markers actually come
//! out of a real shell, with the person's own config loaded, is not — it was
//! checked by running both shells under a pty and counting the sequences, and
//! it is worth re-running by hand after any change to the two files here.

use std::path::{Path, PathBuf};

/// What the startup file is asked to turn on.
///
/// A positive list, and it is unset before the person's own config runs. The
/// earlier shape everywhere is a negative, exported switch — `NO_MARKERS=1` —
/// which lives in the terminal's environment and is inherited by everything
/// launched from it. With an allowlist, a stale or inherited value can only
/// ever mean *fewer* features, never more.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feature {
    /// OSC 133 A/C/D: where a prompt begins, where a command begins, and what
    /// it exited with.
    Marks,
    /// A private OSC saying the prompt is drawn and accepting input. It is the
    /// starting gun for delivering a first command: without it, a slow prompt
    /// swallows whatever was sent before the shell owned the terminal.
    Ready,
    /// A private OSC carrying the shell's own pid, so a pane can be tied to a
    /// process without guessing from the process table.
    Identity,
}

impl Feature {
    /// The word the startup file matches on.
    pub fn word(self) -> &'static str {
        match self {
            Self::Marks => "marks",
            Self::Ready => "ready",
            Self::Identity => "identity",
        }
    }
}

/// The variable carrying the request. Named, not guessed at, in one place.
pub const FEATURES_ENV: &str = "DEVPIT_SHELL_FEATURES";

/// The private OSC the shell emits once its prompt is up.
pub const READY_MARK: &str = "devpit-shell-ready";
/// The private OSC prefix carrying the shell's pid.
pub const IDENTITY_MARK: &str = "devpit-shell-start";

pub fn encode(features: &[Feature]) -> String {
    features
        .iter()
        .map(|feature| feature.word())
        .collect::<Vec<_>>()
        .join(",")
}

/// Which shell this is, by the last segment of its path.
///
/// `/usr/bin/zsh`, `/bin/zsh` and a bare `zsh` are one shell; a login shell
/// spelled `-zsh` is too, which is why the leading dash is dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Bash,
    Zsh,
    Fish,
    /// A shell we have no startup file for. It runs unwrapped rather than
    /// wrongly: a `--rcfile` handed to `nu` is an error, not a fallback.
    Other,
}

pub fn kind_of(shell: &str) -> Kind {
    let leaf = shell.rsplit(['/', '\\']).next().unwrap_or(shell);
    match leaf.trim_start_matches('-') {
        "bash" => Kind::Bash,
        "zsh" => Kind::Zsh,
        "fish" => Kind::Fish,
        _ => Kind::Other,
    }
}

/// The shell to start: what the person has chosen, or a sane one.
pub fn preferred(env_shell: Option<&str>) -> String {
    match env_shell {
        Some(shell) if !shell.trim().is_empty() => shell.to_owned(),
        _ => "/bin/bash".to_owned(),
    }
}

/// How to launch a shell with the startup file in place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launch {
    pub program: String,
    pub args: Vec<String>,
    /// Environment to add, not to replace: the caller merges it.
    pub env: Vec<(String, String)>,
}

/// The launch for a shell, given where the startup files were written.
///
/// An empty feature list is never wrapped: there would be nothing for the
/// startup file to do, and reading the person's config through ours is a cost
/// with no return.
pub fn launch(shell: &str, root: &Path, features: &[Feature], env_zdotdir: Option<&str>) -> Launch {
    let plain = Launch {
        program: shell.to_owned(),
        args: Vec::new(),
        env: Vec::new(),
    };
    if features.is_empty() {
        return plain;
    }

    match kind_of(shell) {
        Kind::Bash => Launch {
            program: shell.to_owned(),
            args: vec![
                "--rcfile".to_owned(),
                root.join("bash").join("rcfile").to_string_lossy().into(),
            ],
            env: vec![(FEATURES_ENV.to_owned(), encode(features))],
        },
        Kind::Zsh => {
            // `-l` because the person's login files are where their PATH is.
            let mut env = vec![
                (
                    "ZDOTDIR".to_owned(),
                    root.join("zsh").to_string_lossy().into(),
                ),
                (FEATURES_ENV.to_owned(), encode(features)),
            ];
            // Their own ZDOTDIR is carried across so our .zshenv can hand it
            // back before sourcing theirs. Losing it would make zsh read no
            // config of theirs at all.
            if let Some(theirs) = env_zdotdir.filter(|value| !value.trim().is_empty()) {
                env.push(("DEVPIT_ORIG_ZDOTDIR".to_owned(), theirs.to_owned()));
            }
            Launch {
                program: shell.to_owned(),
                args: vec!["-l".to_owned()],
                env,
            }
        }
        // `--init-command` runs after their own config, so ours is added to
        // it rather than read in its place. Single-quoted for fish, which
        // takes a backslash-escaped quote inside one.
        Kind::Fish => {
            let file = root.join("fish").join("init.fish");
            let quoted = file
                .to_string_lossy()
                .replace('\\', "\\\\")
                .replace('\'', "\\'");
            Launch {
                program: shell.to_owned(),
                args: vec!["--init-command".to_owned(), format!("source '{quoted}'")],
                env: vec![(FEATURES_ENV.to_owned(), encode(features))],
            }
        }
        Kind::Other => plain,
    }
}

/// The files a wrapped launch needs, as `(relative path, contents)`.
pub fn files() -> Vec<(PathBuf, String)> {
    vec![
        (PathBuf::from("bash").join("rcfile"), BASH.to_owned()),
        (PathBuf::from("zsh").join(".zshenv"), ZSH.to_owned()),
        (PathBuf::from("fish").join("init.fish"), FISH.to_owned()),
    ]
}

/// The startup file for bash.
///
/// `PROMPT_COMMAND` for the prompt boundary and the exit code, a chained
/// `DEBUG` trap for the command start. The trap is chained rather than set
/// because bash-preexec and starship install their own at the first prompt;
/// re-taking it every prompt and calling theirs from ours is what keeps both
/// working.
const BASH: &str = include_str!("shell/rcfile.bash");

/// The startup file for zsh.
///
/// Everything is deferred to a `precmd` that runs *after* the person's own
/// config, because that config can replace `precmd_functions` wholesale. The
/// hooks are prepended to the existing arrays rather than assigned over them.
const ZSH: &str = include_str!("shell/zshenv.zsh");

/// The startup file for fish.
///
/// fish 4 marks its own prompts; there only the line that ran is added. An
/// older fish gets the marks from here, through its `fish_prompt`,
/// `fish_preexec` and `fish_postexec` events.
const FISH: &str = include_str!("shell/init.fish");

/// Where the startup files live, keyed by a hash of the exact bytes.
///
/// Content-addressed on purpose: "the directory is there and its files are
/// non-empty" then proves *this* build wrote them, so a launch never has to
/// rewrite a file another terminal is reading. A build that changes a wrapper
/// gets a different directory and leaves the old one to the shells still using
/// it.
pub fn root_for(base: &Path) -> PathBuf {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for (path, body) in files() {
        for byte in path.to_string_lossy().bytes().chain(body.bytes()) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    base.join(format!("shell-{hash:016x}"))
}

/// Write the startup files under `root`, unless they are already there.
///
/// A marker file goes in the zsh directory: the wrapper checks for it before
/// honouring an inherited `ZDOTDIR`, so a stale value pointing back at us
/// cannot make zsh read our config as if it were theirs.
pub fn install(root: &Path) -> std::io::Result<()> {
    if complete(root) {
        return Ok(());
    }
    for (relative, body) in files() {
        let path = root.join(&relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, body)?;
    }
    std::fs::write(root.join("zsh").join(".devpit-shell-wrapper"), b"")?;
    Ok(())
}

/// Every file present and non-empty.
///
/// Non-empty and not merely present: a half-written `.zshenv` is a file zsh
/// reads happily and learns nothing from, and pointing `ZDOTDIR` at it costs
/// the person their whole configuration.
pub fn complete(root: &Path) -> bool {
    files().iter().all(|(relative, _)| {
        std::fs::metadata(root.join(relative)).is_ok_and(|meta| meta.len() > 0)
    })
}

#[cfg(test)]
#[path = "shell_tests.rs"]
mod shell_tests;
