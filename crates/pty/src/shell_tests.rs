use super::*;

#[test]
fn a_shell_is_known_by_its_last_segment() {
    assert_eq!(kind_of("/usr/bin/zsh"), Kind::Zsh);
    assert_eq!(kind_of("bash"), Kind::Bash);
    /* A login shell is spelled with a leading dash in the process table. */
    assert_eq!(kind_of("-zsh"), Kind::Zsh);
    assert_eq!(kind_of("/usr/bin/fish"), Kind::Other);
}

#[test]
fn a_shell_we_have_no_startup_file_for_runs_unwrapped() {
    /* Rather than wrongly: `--rcfile` handed to fish is an error, not a
    fallback. */
    let it = launch("/usr/bin/fish", Path::new("/w"), &[Feature::Marks], None);
    assert!(it.args.is_empty());
    assert!(it.env.is_empty());
}

#[test]
fn asking_for_nothing_is_never_wrapped() {
    let it = launch("/bin/bash", Path::new("/w"), &[], None);
    assert!(it.args.is_empty(), "there is nothing for a wrapper to do");
}

#[test]
fn bash_is_wrapped_through_its_rcfile() {
    let it = launch("/bin/bash", Path::new("/w"), &[Feature::Marks], None);
    assert_eq!(it.args[0], "--rcfile");
    assert!(it.args[1].ends_with("/w/bash/rcfile"), "{}", it.args[1]);
    assert_eq!(it.env, [(FEATURES_ENV.to_owned(), "marks".to_owned())]);
}

#[test]
fn zsh_is_wrapped_through_zdotdir_and_keeps_theirs() {
    /* Losing their ZDOTDIR would make zsh read no config of theirs at all. */
    let it = launch(
        "/bin/zsh",
        Path::new("/w"),
        &[Feature::Marks, Feature::Ready],
        Some("/home/j/.config/zsh"),
    );
    assert_eq!(it.args, ["-l"]);
    let env: std::collections::HashMap<_, _> = it.env.into_iter().collect();
    assert_eq!(env["ZDOTDIR"], "/w/zsh");
    assert_eq!(env["DEVPIT_ORIG_ZDOTDIR"], "/home/j/.config/zsh");
    assert_eq!(env[FEATURES_ENV], "marks,ready");
}

#[test]
fn an_empty_zdotdir_is_not_carried_across() {
    let it = launch("/bin/zsh", Path::new("/w"), &[Feature::Marks], Some("  "));
    assert!(!it.env.iter().any(|(key, _)| key == "DEVPIT_ORIG_ZDOTDIR"));
}

#[test]
fn the_root_changes_when_a_wrapper_does() {
    /* Content-addressed: a build that changes a wrapper writes elsewhere and
    leaves the old one to the shells still reading it. */
    let base = Path::new("/base");
    assert_eq!(root_for(base), root_for(base));
    assert!(root_for(base).to_string_lossy().contains("shell-"));
}

#[test]
fn installing_writes_every_file_and_the_marker() {
    let dir = std::env::temp_dir().join(format!("devpit-shell-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    assert!(!complete(&dir), "nothing is installed yet");
    install(&dir).expect("write the wrappers");
    assert!(complete(&dir));
    /* The marker is what stops an inherited ZDOTDIR pointing back at us from
    making zsh read our config as if it were theirs. */
    assert!(dir.join("zsh").join(".devpit-shell-wrapper").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_wrapper_destroys_the_request_before_their_config_runs() {
    /* An exported switch lives in the terminal's environment and is inherited
    by everything launched from it, including another devpit. */
    for (_, body) in files() {
        let unset = body
            .find("DEVPIT_SHELL_FEATURES")
            .zip(body.find("unset DEVPIT_SHELL_FEATURES"))
            .expect("the wrapper reads and unsets the request");
        assert!(unset.0 <= unset.1, "it is read before it is unset");
        let sources_theirs = body.find(".zshenv\"").or_else(|| body.find(".bashrc"));
        if let Some(theirs) = sources_theirs {
            assert!(
                unset.1 < theirs,
                "the request dies before their config runs"
            );
        }
    }
}

#[test]
fn a_wrapper_chains_rather_than_replaces() {
    let bash = &files()[0].1;
    assert!(
        bash.contains("__devpit_their_trap"),
        "their DEBUG trap keeps running"
    );
    assert!(
        bash.contains("${PROMPT_COMMAND[@]+"),
        "their PROMPT_COMMAND is kept"
    );
    let zsh = &files()[1].1;
    assert!(
        zsh.contains("__devpit_prev_line_init"),
        "their zle-line-init keeps running"
    );
    assert!(
        zsh.contains("${preexec_functions[@]}"),
        "their preexec hooks are kept"
    );
}

#[test]
fn a_wrapper_speaks_through_tmux_when_it_is_inside_one() {
    /* tmux eats every escape sequence it does not itself understand, OSC 133
    among them, so a shell reporting its prompt boundaries reports them to
    tmux and to nobody else. The passthrough DCS is the only form that
    reaches the client — measured both ways. */
    for (path, body) in files() {
        assert!(
            body.contains("${TMUX:-}"),
            "{} does not ask whether it is inside tmux",
            path.display()
        );
        assert!(
            body.contains("Ptmux;"),
            "{} never wraps a sequence for tmux",
            path.display()
        );
    }
}
