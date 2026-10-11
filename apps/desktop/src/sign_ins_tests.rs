use std::path::Path;

use devpit_rpc::{Declared, EnvVar};

use super::lacks_in;
use crate::installations::Found;

const NOW: f64 = 2_000.0;
const SIGNED_IN: &str =
    r#"{"claudeAiOauth":{"accessToken":"a","refreshToken":"r","refreshTokenExpiresAt":9000}}"#;
const LAPSED: &str =
    r#"{"claudeAiOauth":{"accessToken":"a","refreshToken":"r","refreshTokenExpiresAt":1000}}"#;

fn holding(dir: &Path, credentials: &str) {
    std::fs::write(dir.join(".credentials.json"), credentials).expect("write");
}

/// The default directory, then one profile's own.
fn installations(default: &Path, own: &Path, label: &str) -> Vec<Found> {
    vec![
        Found {
            directory: default.to_path_buf(),
            said: None,
            profiles: Vec::new(),
            ids: Vec::new(),
            default: true,
        },
        Found {
            directory: own.to_path_buf(),
            said: Some(own.display().to_string()),
            profiles: vec![label.to_owned()],
            ids: vec![format!("p-{label}")],
            default: false,
        },
    ]
}

#[test]
fn an_expired_login_holds_the_start_with_the_command_that_fixes_it() {
    if cfg!(target_os = "macos") {
        return; // The Keychain holds it there: nothing to read, nothing held.
    }
    let (default, work) = (
        tempfile::tempdir().expect("dir"),
        tempfile::tempdir().expect("dir"),
    );
    holding(default.path(), SIGNED_IN);
    holding(work.path(), LAPSED);
    let all = installations(default.path(), work.path(), "Work");

    let missing = lacks_in(&all, &[], "p-Work", NOW).expect("expired");
    assert_eq!(
        missing.what,
        "the Claude account Work's sign-in has expired"
    );
    assert_eq!(
        missing.command,
        format!(
            "CLAUDE_CONFIG_DIR='{}' claude auth login",
            work.path().display()
        )
    );
    let held = crate::shell_launch::held(vec![missing.clone()]).expect("held");
    assert_eq!(held.line, None);
    assert_eq!(held.missing, [missing]);

    // The bare CLI runs against the default directory, which is signed in.
    assert_eq!(lacks_in(&all, &[], "claude", NOW), None);
    assert_eq!(crate::shell_launch::held(Vec::new()), None);
}

#[test]
fn a_signed_out_default_account_says_the_plain_login() {
    if cfg!(target_os = "macos") {
        return;
    }
    let (default, work) = (
        tempfile::tempdir().expect("dir"),
        tempfile::tempdir().expect("dir"),
    );
    holding(work.path(), SIGNED_IN);
    let all = installations(default.path(), work.path(), "Work");
    let missing = lacks_in(&all, &[], "claude", NOW).expect("signed out");
    assert_eq!(missing.what, "the Claude account Claude Code is signed out");
    assert_eq!(missing.command, "claude auth login");
    assert_eq!(lacks_in(&all, &[], "p-Work", NOW), None);
}

#[test]
fn an_account_on_another_provider_has_no_sign_in_to_lapse() {
    let (default, glm) = (
        tempfile::tempdir().expect("dir"),
        tempfile::tempdir().expect("dir"),
    );
    // Its directory holds no Claude sign-in at all, and needs none.
    let all = installations(default.path(), glm.path(), "glm");
    let gateway = Declared {
        id: "p-glm".to_owned(),
        label: "glm".to_owned(),
        base: "claude".to_owned(),
        command: String::new(),
        args: Vec::new(),
        env: vec![EnvVar {
            name: "ANTHROPIC_BASE_URL".to_owned(),
            value: "https://api.z.ai/api/anthropic".to_owned(),
        }],
        models: Vec::new(),
    };
    assert_eq!(lacks_in(&all, &[gateway], "p-glm", NOW), None);
}
