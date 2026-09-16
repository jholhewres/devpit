//! What the policy has to refuse, tested against policies rather than files.

use super::*;

const OURS: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; \
                    img-src 'self' data: blob:; font-src 'self' data:; \
                    connect-src 'self' ipc: http://ipc.localhost; worker-src 'self' blob:; \
                    frame-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'";

#[test]
fn the_policy_this_app_ships_is_accepted() {
    assert_eq!(refusals(OURS, &ALLOWED), Vec::<String>::new());
}

/// The four ways a policy stops being one.
#[test]
fn a_policy_that_lets_anything_through_is_refused() {
    // Adding a scheme is the whole of `https:` — the sabotage the story names.
    let loosened = OURS.replace("connect-src 'self'", "connect-src 'self' https:");
    assert!(
        refusals(&loosened, &ALLOWED)
            .iter()
            .any(|said| said.contains("any host over https:")),
        "{loosened}"
    );

    assert!(refusals(&OURS.replace("'self'", "*"), &ALLOWED)
        .iter()
        .any(|said| said.contains("wildcard")));

    assert!(refusals(
        &OURS.replace("script-src 'self'", "script-src 'self' 'unsafe-eval'"),
        &ALLOWED
    )
    .iter()
    .any(|said| said.contains("unsafe-eval")));

    assert!(refusals(
        &OURS.replace("frame-src 'none'", "frame-src 'self'"),
        &ALLOWED
    )
    .iter()
    .any(|said| said.contains("frame-src names 'self'")));

    // A host nobody allowed, even a quiet-looking one.
    assert!(refusals(
        &OURS.replace("ipc:", "ipc: https://telemetry.example"),
        &ALLOWED
    )
    .iter()
    .any(|said| said.contains("https://telemetry.example")));
}

/// The dev server is allowed where dev runs, and only there.
#[test]
fn the_dev_server_is_named_only_in_the_dev_policy() {
    let dev = OURS.replace(
        "http://ipc.localhost",
        "http://ipc.localhost http://localhost:17800 ws://localhost:17800",
    );
    assert_eq!(refusals(&dev, &ALLOWED_IN_DEV), Vec::<String>::new());
    assert_eq!(
        refusals(&dev, &ALLOWED).len(),
        2,
        "dev hosts passed in the shipped policy"
    );
}

/// The review's cases: both passed a guard that says it forbids injectable
/// script — a policy with no script source at all, and one that lets inline
/// script and data URLs run.
#[test]
fn scripts_have_to_come_from_the_app_itself() {
    let silent = "img-src 'self'; frame-src 'none'; object-src 'none'";
    assert!(
        refusals(silent, &[])
            .iter()
            .any(|why| why.contains("neither script-src nor default-src")),
        "a policy that says nothing about scripts was accepted"
    );

    let inline = "default-src 'self'; script-src 'self' 'unsafe-inline' data:; frame-src 'none'; object-src 'none'";
    assert!(
        refusals(inline, &[])
            .iter()
            .any(|why| why.contains("only 'self' is")),
        "inline script was accepted"
    );

    let fallback = "default-src 'self'; frame-src 'none'; object-src 'none'";
    assert!(
        !refusals(fallback, &[])
            .iter()
            .any(|why| why.contains("script")),
        "default-src 'self' alone is a fine answer for scripts"
    );
}
