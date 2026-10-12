use devpit_rpc::Privacy;

use super::Kept;

#[test]
fn devpits_bot_starts_at_the_minimal_level() {
    assert_eq!(Kept::default().privacy(), Privacy::Minimal);
}

#[test]
fn a_whole_answer_never_goes_through_devpit_app() {
    let kept = Kept {
        privacy: Some(Privacy::Full),
        ..Kept::default()
    };
    assert_eq!(kept.privacy(), Privacy::Titles);
    let answer = "Deployed.\n\nThe staging key was rotated and the old one is in .env.bak.";
    assert_eq!(
        crate::channel_talk::shaped(answer, kept.privacy()),
        vec!["Deployed.…".to_owned()]
    );
    assert_eq!(
        crate::channel_talk::shaped(answer, Kept::default().privacy()),
        vec!["The orchestrator answered: read it in devpit.".to_owned()]
    );
}
