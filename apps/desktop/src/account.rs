//! Contract commands for the account this install is signed in as.
//!
//! The app never sees a password. `account.sign_in` asks the accounts server
//! for a short code, opens the browser at the page that approves it, and then
//! `account.poll` asks until someone has. What comes back is a device token,
//! which lives in `~/.devpit/account-token` at mode 600 and goes out as a
//! bearer header — never to the screen.

use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use devpit_rpc::{Account, Membership, RpcError, SignIn, SignInState};
use serde::Deserialize;

/// Where the accounts server lives. Overridable so a build can be pointed at a
/// local one without editing this file.
fn origin() -> String {
    std::env::var("DEVPIT_ACCOUNT_ORIGIN")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "https://devpit.jhol.dev".to_owned())
}

/// The polling secret of a sign-in in progress.
///
/// In memory and nowhere else: a sign-in that did not finish before the window
/// closed is a sign-in to start again, not state worth keeping.
fn in_flight() -> &'static Mutex<Option<String>> {
    static PENDING: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    PENDING.get_or_init(|| Mutex::new(None))
}

fn client() -> Result<reqwest::Client, RpcError> {
    reqwest::Client::builder()
        // A request that hangs must not hang the pane that made it.
        .timeout(Duration::from_secs(20))
        .user_agent(concat!("devpit/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|err| RpcError::internal(format!("could not build an http client: {err}")))
}

/// `account.read` — who this install is signed in as.
///
/// A token the server refuses is deleted here rather than kept to fail again
/// on every launch, and `expired` says so, because "your session ended" and
/// "you were never signed in" are different sentences.
#[tauri::command]
#[specta::specta]
pub async fn account_read() -> Result<Membership, RpcError> {
    let origin = origin();
    let Some(token) = token::read() else {
        return Ok(Membership {
            account: None,
            origin,
            expired: false,
        });
    };

    let response = client()?
        .get(format!("{origin}/v1/auth/session"))
        .bearer_auth(&token)
        .send()
        .await
        .map_err(unreachable_server)?;

    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        token::forget();
        return Ok(Membership {
            account: None,
            origin,
            expired: true,
        });
    }
    if !response.status().is_success() {
        return Err(RpcError::internal(format!(
            "the accounts server answered {}",
            response.status()
        )));
    }

    let answer: SessionAnswer = response.json().await.map_err(unreadable)?;
    // A 200 with no account means the token names a session that is gone.
    let expired = answer.account.is_none();
    if expired {
        token::forget();
    }
    Ok(Membership {
        account: answer.account,
        origin,
        expired,
    })
}

/// `account.sign_in` — start a sign-in and open the browser on it.
#[tauri::command]
#[specta::specta]
pub async fn account_sign_in() -> Result<SignIn, RpcError> {
    let origin = origin();
    let grant: Grant = client()?
        .post(format!("{origin}/v1/device/start"))
        .json(&serde_json::json!({ "label": label() }))
        .send()
        .await
        .map_err(unreachable_server)?
        .error_for_status()
        .map_err(|err| RpcError::internal(format!("could not start a sign-in: {err}")))?
        .json()
        .await
        .map_err(unreadable)?;

    if let Ok(mut pending) = in_flight().lock() {
        *pending = Some(grant.poll_token.clone());
    }

    // Best effort: if no browser opens, the code is on screen and the url is
    // in the answer, so the person still has a way through.
    browser::open(&grant.verify_url);

    Ok(SignIn {
        user_code: grant.user_code,
        verify_url: grant.verify_url,
        expires_in_seconds: grant.expires_in_seconds,
        interval_seconds: grant.interval_seconds,
    })
}

/// `account.poll` — has the person approved it yet.
#[tauri::command]
#[specta::specta]
pub async fn account_poll() -> Result<SignInState, RpcError> {
    let Some(poll_token) = in_flight().lock().ok().and_then(|pending| pending.clone()) else {
        return Ok(SignInState::Expired);
    };

    let state: PollAnswer = client()?
        .post(format!("{}/v1/device/poll", origin()))
        .json(&serde_json::json!({ "pollToken": poll_token }))
        .send()
        .await
        .map_err(unreachable_server)?
        .error_for_status()
        .map_err(|err| RpcError::internal(format!("could not check the sign-in: {err}")))?
        .json()
        .await
        .map_err(unreadable)?;

    match state {
        PollAnswer::Pending => Ok(SignInState::Waiting),
        PollAnswer::Expired => {
            clear_in_flight();
            Ok(SignInState::Expired)
        }
        PollAnswer::Approved { token, account } => {
            // Written before the screen is told, so a person who sees "signed
            // in" is signed in on the next launch too.
            token::write(&token)?;
            clear_in_flight();
            Ok(SignInState::Signed { account })
        }
    }
}

/// `account.sign_out` — forget the token, and tell the server to as well.
///
/// The local file goes first. A network failure on the way out must not leave
/// someone looking at an account they asked to leave.
#[tauri::command]
#[specta::specta]
pub async fn account_sign_out() -> Result<Membership, RpcError> {
    let origin = origin();
    let token = token::read();
    token::forget();
    clear_in_flight();

    if let Some(token) = token {
        if let Ok(client) = client() {
            let _ = client
                .post(format!("{origin}/v1/auth/logout"))
                .bearer_auth(token)
                .send()
                .await;
        }
    }
    Ok(Membership {
        account: None,
        origin,
        expired: false,
    })
}

fn clear_in_flight() {
    if let Ok(mut pending) = in_flight().lock() {
        *pending = None;
    }
}

/// What this install calls itself on the approval screen and in the device
/// list. The platform is in it because "devpit on Linux" is what tells two of
/// someone's machines apart.
fn label() -> String {
    format!(
        "devpit {} ({})",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS
    )
}

fn unreachable_server(err: reqwest::Error) -> RpcError {
    // The one failure worth a sentence of its own: the person is offline, or
    // the server is, and neither is something they can fix by retrying twice.
    RpcError::internal(format!("could not reach the accounts server: {err}"))
}

fn unreadable(err: reqwest::Error) -> RpcError {
    RpcError::internal(format!(
        "the accounts server said something unexpected: {err}"
    ))
}

#[derive(Deserialize)]
struct SessionAnswer {
    account: Option<Account>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Grant {
    user_code: String,
    verify_url: String,
    poll_token: String,
    expires_in_seconds: u32,
    interval_seconds: u32,
}

#[derive(Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum PollAnswer {
    Pending,
    Approved { token: String, account: Account },
    Expired,
}

pub(crate) mod token {
    //! The device token on disk.

    use std::path::PathBuf;

    use devpit_rpc::RpcError;

    fn path() -> Option<PathBuf> {
        devpit_core::Store::root()
            .ok()
            .map(|root| root.join("account-token"))
    }

    pub fn read() -> Option<String> {
        let raw = std::fs::read_to_string(path()?).ok()?;
        let token = raw.trim().to_owned();
        (!token.is_empty()).then_some(token)
    }

    pub fn write(token: &str) -> Result<(), RpcError> {
        let path = path().ok_or_else(|| RpcError::internal("no home directory to write to"))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| RpcError::internal(format!("could not create {parent:?}: {err}")))?;
        }
        devpit_core::home::write_private(&path, token.as_bytes())
            .map_err(|err| RpcError::internal(format!("could not write the token: {err}")))?;
        Ok(())
    }

    pub fn forget() {
        if let Some(path) = path() {
            let _ = std::fs::remove_file(path);
        }
    }
}

mod browser {
    //! Opening a url in whatever the person uses.

    /// Arguments, never a command line: a url reaching a shell as text is a
    /// url that can carry shell syntax.
    pub fn open(url: &str) {
        // Set where there is no browser to open — a test, or a headless box.
        if std::env::var("DEVPIT_ACCOUNT_NO_BROWSER").is_ok_and(|value| value == "1") {
            return;
        }

        let (program, args): (&str, &[&str]) = if cfg!(target_os = "macos") {
            ("open", &[])
        } else if cfg!(target_os = "windows") {
            ("cmd", &["/C", "start", ""])
        } else {
            ("xdg-open", &[])
        };

        let opened = devpit_pty::host_env::command(program)
            .args(args)
            .arg(url)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();

        if let Err(err) = opened {
            // Not fatal: the screen shows the code and the address as well.
            tracing_note(&format!("could not open a browser with {program}: {err}"));
        }
    }

    fn tracing_note(message: &str) {
        eprintln!("devpit: {message}");
    }
}

#[cfg(test)]
#[path = "account_tests.rs"]
mod tests;
