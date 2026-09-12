//! Recognition, read off argument vectors that were actually observed.

use super::*;

fn argv(line: &str) -> Vec<String> {
    line.split_whitespace().map(ToOwned::to_owned).collect()
}

/// The reason this file exists: every JavaScript agent runs as `node`, so the
/// executable's own name is never the answer.
#[test]
fn a_javascript_agent_is_named_by_its_script() {
    let found = recognise(&argv("node /home/j/.local/bin/claude --resume"));
    assert_eq!(found.map(|one| one.id), Some("claude"));
}

/// A binary that is its own program needs no unwrapping.
#[test]
fn a_native_agent_is_named_by_itself() {
    assert_eq!(recognise(&argv("codex")).map(|one| one.id), Some("codex"));
    assert_eq!(
        recognise(&argv("/usr/local/bin/opencode run")).map(|one| one.id),
        Some("opencode")
    );
}

/// Codex ships a binary named after the platform it was built for.
#[test]
fn a_per_platform_binary_is_still_its_agent() {
    assert_eq!(
        recognise(&argv("codex-aarch64-apple-darwin")).map(|one| one.id),
        Some("codex")
    );
}

/// The class of mistake this is built to avoid: a prompt naming another agent
/// must not rename the pane. `claude` is running, whatever it was asked.
#[test]
fn a_prompt_that_names_an_agent_is_not_that_agent() {
    let found = recognise(&argv("node /opt/claude compare opencode and codex"));
    assert_eq!(found.map(|one| one.id), Some("claude"));
}

/// An interpreter given its source on the command line is running nothing
/// with a name, and must not be read as the next word on the line.
#[test]
fn an_interpreter_with_inline_source_names_nothing() {
    assert_eq!(program_of(&argv("node -e codex")), None);
    assert!(recognise(&argv("node -e claude")).is_none());
}

/// An option that swallows the argument after it must not have that argument
/// read as the program.
#[test]
fn an_option_that_takes_a_value_hides_it() {
    assert_eq!(
        program_of(&argv("node --require /tmp/hook.js /opt/bin/claude")).as_deref(),
        Some("claude"),
    );
    // Nothing is left after the value, so the interpreter is what is running.
    assert_eq!(
        program_of(&argv("node -r /tmp/codex.js")).as_deref(),
        Some("node")
    );
    assert!(recognise(&argv("node -r /tmp/codex.js")).is_none());
}

/// A script keeps the interpreter's extension on disk and loses it as a name.
#[test]
fn a_script_is_named_without_its_extension() {
    assert_eq!(
        program_of(&argv("node /opt/cli/claude.mjs")).as_deref(),
        Some("claude")
    );
}

/// A login shell wears a leading dash and is still a shell sitting idle.
#[test]
fn a_shell_at_its_prompt_is_not_an_agent() {
    assert_eq!(program_of(&argv("-zsh")).as_deref(), Some("zsh"));
    assert!(idle_shell("zsh"));
    assert!(recognise(&argv("-zsh")).is_none());
}

/// An ordinary command is not an agent and keeps its own name.
#[test]
fn an_ordinary_command_is_neither() {
    assert_eq!(program_of(&argv("cargo test")).as_deref(), Some("cargo"));
    assert!(recognise(&argv("cargo test")).is_none());
    assert!(!idle_shell("cargo"));
}

/// The menu and the reader are the same list, so every launchable agent is
/// recognisable by the command the menu would type.
#[test]
fn everything_the_menu_starts_is_recognised_when_it_runs() {
    for one in KNOWN {
        let running = recognise(&argv(one.launch));
        assert_eq!(
            running.map(|found| found.id),
            Some(one.id),
            "{} starts as `{}` and comes back as something else",
            one.id,
            one.launch
        );
    }
}

/// Ids are how the screen keys an icon, so two agents may not share one.
#[test]
fn every_agent_has_its_own_id() {
    for (at, one) in KNOWN.iter().enumerate() {
        assert!(
            !KNOWN[..at].iter().any(|before| before.id == one.id),
            "{} is listed twice",
            one.id
        );
        assert!(known(one.id).is_some());
    }
}

/// Nothing to read is nothing recognised, rather than a panic.
#[test]
fn an_empty_command_line_is_nothing() {
    assert_eq!(program_of(&[]), None);
    assert!(recognise(&[]).is_none());
}
