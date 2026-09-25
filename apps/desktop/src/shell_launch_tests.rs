use std::collections::HashMap;

use super::*;

fn pane(profile: &str) -> devpit_tmux::Running {
    devpit_tmux::Running {
        leaf_id: "w1".to_owned(),
        command: "node".to_owned(),
        tty: "/dev/pts/7".to_owned(),
        profile: profile.to_owned(),
    }
}

fn names(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(id, label)| ((*id).to_owned(), (*label).to_owned()))
        .collect()
}

/*
 * What a pane is called.
 *
 * Two sources, and which wins is the point. Reading the process answers for
 * every terminal, including the ones devpit never touched, and it cannot tell
 * `glm` from `claude2` — same binary, same arguments, different environment.
 * The record answers only for panes devpit started, and for those it is exact.
 */

#[test]
fn a_pane_devpit_started_is_called_what_the_person_called_it() {
    let said = named(&[], &names(&[("01JGLM", "GLM")]), pane("01JGLM"));
    assert_eq!(said.label, "GLM");
}

#[test]
fn a_pane_devpit_did_not_start_falls_back_to_reading_the_process() {
    // No record, so the answer is whatever the process looks like — which is
    // what every row did before profiles existed.
    let said = named(&[], &names(&[("01JGLM", "GLM")]), pane(""));
    assert_eq!(said.label, "node");
}

#[test]
fn a_record_pointing_at_a_profile_that_is_gone_falls_back_too() {
    // Deleting a profile is refused while a step uses it, but nothing stops
    // somebody deleting one with a terminal still open on it.
    let said = named(&[], &names(&[]), pane("01JDELETED"));
    assert_eq!(said.label, "node");
}

#[test]
fn the_record_does_not_make_an_idle_shell_look_busy() {
    // A profile's terminal that has been quit back to a prompt is idle, and
    // the record outlives the agent that was in it.
    let mut quiet = pane("01JGLM");
    quiet.command = "zsh".to_owned();
    let said = named(&[], &names(&[("01JGLM", "GLM")]), quiet);
    assert!(!said.busy);
}

#[test]
fn only_a_bare_name_is_asked_of_the_shell() {
    // The probe pastes these into a line of shell, so anything needing
    // quoting is left out rather than escaped.
    assert!(is_a_bare_name("claude"));
    assert!(is_a_bare_name("cursor-agent"));
    assert!(is_a_bare_name("python3.11"));
    assert!(!is_a_bare_name("rm -rf /"));
    assert!(!is_a_bare_name("a;b"));
    assert!(!is_a_bare_name(""));
}

#[test]
fn the_program_of_a_launch_line_is_its_first_word() {
    assert_eq!(program_of("cursor-agent"), "cursor-agent");
    assert_eq!(program_of("orca claude-teams"), "orca");
}
