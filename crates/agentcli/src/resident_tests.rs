use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;

use devpit_rpc::Part;

use super::{Heard, Resident};
use crate::driver::driver;
use crate::talk::Say;

/// A CLI that answers each line it is given with a turn, and after the first
/// one wakes by itself for a second — the way a message from another session
/// wakes the real one.
const STAND_IN: &str = r#"#!/bin/sh
answer() {
  printf '{"type":"system","subtype":"init","session_id":"s-1"}\n'
  printf '{"type":"assistant","message":{"id":"m-%s","role":"assistant","content":[{"type":"text","text":"%s"}]},"session_id":"s-1"}\n' "$1" "$1"
  printf '{"type":"result","subtype":"success","session_id":"s-1","total_cost_usd":0}\n'
}
read -r _first
answer asked
sleep 0.3
answer woken
while read -r _next; do answer again; done
"#;

fn turn(command: &str) -> Say<'_> {
    Say {
        command,
        env: &[],
        prompt: "",
        cwd: Path::new("/"),
        model: None,
        budget_usd: None,
        session_id: None,
        fork_at: None,
        permission: None,
        settings: None,
        mcp_config: None,
        effort: None,
        control: None,
        on_session: None,
        add_dirs: &[],
    }
}

/// Starts the stand-in, with the retry the turn tests use: a script written a
/// moment ago can be "text file busy" while another test forks with it open.
fn started(command: &str, tell: mpsc::Sender<String>) -> Resident {
    (0..5)
        .find_map(|_| {
            let tell = tell.clone();
            match Resident::start(
                driver("claude").expect("driver"),
                &turn(command),
                Arc::new(|_: &str| {}),
                move |one| {
                    if let Some(line) = said(&one) {
                        let _ = tell.send(line);
                    }
                },
            ) {
                Err(crate::AgentError::NotInstalled) => {
                    std::thread::sleep(Duration::from_millis(50));
                    None
                }
                other => Some(other.expect("started")),
            }
        })
        .expect("the stand-in started within five tries")
}

fn said(heard: &Heard) -> Option<String> {
    match heard {
        Heard::Part(Part::Text { text, .. }) => Some(text.clone()),
        Heard::Ended(_) => Some("<ended>".to_owned()),
        Heard::Gone => Some("<gone>".to_owned()),
        _ => None,
    }
}

#[test]
fn it_stays_between_turns_and_is_heard_when_woken() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cli = dir.path().join("fake-claude");
    std::fs::write(&cli, STAND_IN).expect("script");
    std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let command = cli.to_string_lossy().into_owned();

    let (tell, heard) = mpsc::channel();
    let resident = started(&command, tell);

    let next = || {
        heard
            .recv_timeout(Duration::from_secs(5))
            .expect("heard in time")
    };
    assert!(resident.say("hello"));
    assert_eq!((next(), next()), ("asked".to_owned(), "<ended>".to_owned()));
    // Nobody asked: the process woke on its own and was still heard.
    assert_eq!((next(), next()), ("woken".to_owned(), "<ended>".to_owned()));
    // And it is still there to be asked again.
    assert!(resident.say("again"));
    assert_eq!((next(), next()), ("again".to_owned(), "<ended>".to_owned()));

    resident.close();
    assert_eq!(next(), "<gone>");
}

/// A process that dies halfway through a turn is reported gone, with no end
/// frame: the turn failed, and whoever waits on it hears so instead of waiting.
#[test]
fn a_process_that_dies_mid_turn_is_heard_as_gone() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cli = dir.path().join("fake-claude");
    std::fs::write(
        &cli,
        "#!/bin/sh\nread -r _first\nprintf '{\"type\":\"assistant\",\"message\":{\"id\":\"m-1\",\"role\":\"assistant\",\"content\":[{\"type\":\"text\",\"text\":\"half\"}]},\"session_id\":\"s-1\"}\\n'\nexit 3\n",
    )
    .expect("script");
    std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let command = cli.to_string_lossy().into_owned();

    let (tell, heard) = mpsc::channel();
    let resident = started(&command, tell);
    assert!(resident.say("go"));
    let next = || {
        heard
            .recv_timeout(Duration::from_secs(5))
            .expect("heard in time")
    };
    assert_eq!((next(), next()), ("half".to_owned(), "<gone>".to_owned()));
}
