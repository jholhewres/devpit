use super::*;

#[test]
fn the_label_names_the_app_and_the_machine() {
    // It reaches the approval screen and the device list, where the whole job
    // is telling two of someone's machines apart.
    let label = label();
    assert!(label.starts_with("devpit "), "{label}");
    assert!(label.contains(std::env::consts::OS), "{label}");
}

#[test]
fn the_origin_can_be_pointed_somewhere_else() {
    // The default is production; a local server is how this gets tested at
    // all, so it has to be reachable without editing the file.
    let restore = std::env::var("DEVPIT_ACCOUNT_ORIGIN").ok();

    std::env::set_var("DEVPIT_ACCOUNT_ORIGIN", "http://127.0.0.1:8787");
    assert_eq!(origin(), "http://127.0.0.1:8787");

    // An empty value is not a choice, it is an unset variable spelled badly.
    std::env::set_var("DEVPIT_ACCOUNT_ORIGIN", "");
    assert_eq!(origin(), "https://devpit.jhol.dev");

    match restore {
        Some(value) => std::env::set_var("DEVPIT_ACCOUNT_ORIGIN", value),
        None => std::env::remove_var("DEVPIT_ACCOUNT_ORIGIN"),
    }
}

#[test]
fn a_poll_answer_is_read_the_way_the_server_writes_it() {
    // The tag and the case are the contract. Getting either wrong means the
    // app waits forever on a sign-in that already succeeded.
    let approved: PollAnswer = serde_json::from_value(serde_json::json!({
        "state": "approved",
        "token": "a-device-token",
        "account": {
            "id": "01ABC",
            "email": "person@example.com",
            "name": null,
            "createdAt": "2026-09-10T00:00:00Z"
        }
    }))
    .expect("read an approval");

    match approved {
        PollAnswer::Approved { token, account } => {
            assert_eq!(token, "a-device-token");
            assert_eq!(account.email, "person@example.com");
        }
        _ => panic!("an approval read as something else"),
    }

    assert!(matches!(
        serde_json::from_value::<PollAnswer>(serde_json::json!({ "state": "pending" }))
            .expect("read pending"),
        PollAnswer::Pending
    ));
    assert!(matches!(
        serde_json::from_value::<PollAnswer>(serde_json::json!({ "state": "expired" }))
            .expect("read expired"),
        PollAnswer::Expired
    ));
}

#[test]
fn a_grant_is_read_from_the_camel_case_the_server_sends() {
    let grant: Grant = serde_json::from_value(serde_json::json!({
        "userCode": "WXYZ-2345",
        "verifyUrl": "https://devpit.jhol.dev/link?code=WXYZ-2345",
        "pollToken": "a-poll-token",
        "expiresInSeconds": 900,
        "intervalSeconds": 2
    }))
    .expect("read a grant");

    assert_eq!(grant.user_code, "WXYZ-2345");
    assert_eq!(grant.interval_seconds, 2);
}

/// The sign-in end to end, against a real accounts server.
///
/// It plays both halves: the app calls the same functions the window calls,
/// and the browser's part is plain HTTP. What it proves is the thing no unit
/// test can — that the two programs agree on the wire.
///
///     cd ../devpit-app && make dev
///     DEVPIT_ACCOUNT_ORIGIN=http://127.0.0.1:8787 \
///       cargo test -p devpit-desktop account -- --ignored --test-threads=1
#[tokio::test]
#[ignore = "needs an accounts server at DEVPIT_ACCOUNT_ORIGIN"]
async fn the_app_signs_in_through_the_browser_and_stays_signed_in() {
    std::env::set_var("DEVPIT_ACCOUNT_NO_BROWSER", "1");

    // Whatever a previous run left behind is not this run's answer.
    let _ = account_sign_out().await;

    let before = account_read().await.expect("read");
    assert!(before.account.is_none(), "starts signed out");

    let grant = account_sign_in().await.expect("start");
    assert!(
        grant.user_code.contains('-'),
        "a code a person can read: {}",
        grant.user_code
    );
    assert!(
        grant.verify_url.contains(&grant.user_code),
        "the url carries the code"
    );
    assert!(
        grant.interval_seconds >= 1,
        "the server sets the polling interval"
    );

    assert!(
        matches!(account_poll().await.expect("poll"), SignInState::Waiting),
        "nobody has approved it yet"
    );

    approve_as_a_person(&grant.user_code).await;

    let signed = account_poll().await.expect("poll again");
    let SignInState::Signed { account } = signed else {
        panic!("the sign-in did not complete: {signed:?}");
    };
    assert!(account.email.starts_with("device-test-"));

    // The token is on disk now, so a fresh read — what the next launch does —
    // finds the same person without anyone signing in again.
    let after = account_read().await.expect("read again");
    assert_eq!(after.account.expect("signed in").id, account.id);
    assert!(!after.expired);

    // And signing out reaches the server, not just the local file.
    let out = account_sign_out().await.expect("sign out");
    assert!(out.account.is_none());
    assert!(
        account_read()
            .await
            .expect("read once more")
            .account
            .is_none(),
        "still signed out after the token is gone"
    );
}

/// The browser's half: register someone, then approve the code they were shown.
#[cfg(test)]
async fn approve_as_a_person(user_code: &str) {
    let client = reqwest::Client::builder()
        .cookie_store(true)
        .build()
        .expect("build a browser");
    let email = format!("device-test-{}@example.com", ulid::Ulid::generate());

    let registered = client
        .post(format!("{}/v1/auth/register", origin()))
        .json(&serde_json::json!({ "email": email, "password": "a long enough password" }))
        .send()
        .await
        .expect("register");
    assert!(
        registered.status().is_success(),
        "register: {}",
        registered.status()
    );

    let approved = client
        .post(format!("{}/v1/device/approve", origin()))
        .json(&serde_json::json!({ "userCode": user_code }))
        .send()
        .await
        .expect("approve");
    assert!(
        approved.status().is_success(),
        "approve: {}",
        approved.status()
    );
}
