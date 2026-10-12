//! Telegram through devpit's own bot, on devpit.app, for a person who would
//! rather not make one. The chat is linked there with a single-use code;
//! notices go with the hub's (`via: telegram`), answers to the hub's bot
//! endpoint, and what the chat writes comes back with the hub's pressed
//! buttons — a `channel_talk::Line` like the person's own bot, with devpit.app
//! in between.
//!
//! What goes through devpit.app is what the person chose: by default only that
//! something waits; names and an answer's first line once they turn titles on.
//! A whole answer never does — that is their own bot's.

use devpit_rpc::{ChannelInfo, Privacy, RpcError, TelegramStatus};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::channel_talk::Heard;
use crate::channels::Action;

/// How long a linking code holds, as devpit.app keeps it.
const CODE_SECS: i64 = 10 * 60;
pub(crate) const TEXT_ONLY: &str =
    "devpit's bot carries text: type it, or link your own bot for voice notes.";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct Kept {
    /// devpit.app last said a chat is linked.
    pub linked: bool,
    pub bot: String,
    pub code: Option<String>,
    pub link: Option<String>,
    pub code_until: i64,
    pub privacy: Option<Privacy>,
    /// The orchestrator the chat talks to, by project.
    pub orchestrator: Option<String>,
}

impl Kept {
    /// How much goes through devpit.app: never the whole answer.
    pub(crate) fn privacy(&self) -> Privacy {
        match self.privacy.unwrap_or(Privacy::Minimal) {
            Privacy::Full => Privacy::Titles,
            other => other,
        }
    }
}

fn file() -> Option<std::path::PathBuf> {
    devpit_core::Store::root()
        .ok()
        .map(|root| root.join("telegram-devpit.json"))
}

pub(crate) fn read() -> Option<Kept> {
    serde_json::from_str(&std::fs::read_to_string(file()?).ok()?).ok()
}

fn write(kept: &Kept) -> Result<(), RpcError> {
    let path = file().ok_or_else(|| RpcError::internal("no home directory to write to"))?;
    let text = serde_json::to_vec(kept).map_err(|err| RpcError::internal(err.to_string()))?;
    devpit_core::home::write_private(&path, &text)
        .map_err(|err| RpcError::internal(err.to_string()))
}

/// A chat is linked through devpit's bot, and this machine is signed in.
pub(crate) fn linked() -> bool {
    read().is_some_and(|kept| kept.linked) && crate::account::token::read().is_some()
}

/// Telegram as a channel, when it goes through devpit's bot.
pub(crate) fn connected() -> Option<ChannelInfo> {
    let kept = read().filter(|_| linked())?;
    Some(ChannelInfo {
        id: crate::telegram::ID.to_owned(),
        label: format!("Telegram, @{} (devpit's bot)", kept.bot),
    })
}

pub(crate) fn titles() -> bool {
    read().is_some_and(|kept| kept.privacy() != Privacy::Minimal)
}

/// A notice for the linked chat, through the hub.
pub(crate) fn send(text: &str, offered: &[Action]) {
    crate::hub::send(text, offered, &["telegram"]);
}

/// What devpit.app said a moment ago: whether the chat is still linked there.
/// Unlinked there — from another machine, or the bot is gone — it is here too.
pub(crate) fn seen(linked_there: bool) {
    let Some(mut kept) = read() else { return };
    if kept.linked == linked_there {
        return;
    }
    kept.linked = linked_there;
    if linked_there {
        kept.code = None;
        kept.link = None;
    }
    let _ = write(&kept);
}

/// What the linked chat wrote, as devpit.app passed it on: to the
/// orchestrator, one after another, in the order written.
pub(crate) fn heard(said: Vec<String>) {
    let Some(app) = crate::telegram::app().get().cloned() else {
        return;
    };
    if said.is_empty() {
        return;
    }
    tauri::async_runtime::spawn(async move {
        for text in said {
            let here = crate::channel_talk::Here(app.clone());
            crate::channel_talk::heard(&Bot, &here, Heard::Text(text)).await;
        }
    });
}

/// devpit's bot, as the linked chat's side of a talk with an orchestrator.
struct Bot;

impl crate::channel_talk::Line for Bot {
    fn id(&self) -> &'static str {
        crate::telegram::ID
    }

    fn privacy(&self) -> Privacy {
        read().unwrap_or_default().privacy()
    }

    fn orchestrator(&self) -> Option<String> {
        read()?.orchestrator
    }

    async fn reply(&self, text: &str, actions: &[Action]) {
        said(text, actions).await;
    }

    async fn fetch(&self, _file: &str, _most: usize) -> Result<Vec<u8>, String> {
        Err(TEXT_ONLY.to_owned())
    }
}

/// [`Bot::reply`]: a message to the linked chat, through devpit.app.
async fn said(text: &str, offered: &[Action]) {
    let (Some(token), Ok(client)) = (crate::account::token::read(), crate::account::client())
    else {
        return;
    };
    let buttons: Vec<_> = offered
        .iter()
        .map(|action| json!({ "action": crate::channel_actions::remember(action), "label": crate::channel_actions::label(action) }))
        .collect();
    let sent = client
        .post(format!(
            "{}/v1/hub/telegram/messages",
            crate::account::origin()
        ))
        .bearer_auth(token)
        .json(&json!({ "text": text, "buttons": buttons }))
        .send()
        .await;
    match sent {
        Ok(answer) if answer.status().is_success() => {}
        Ok(answer) => devpit_core::reports::background(
            "telegram",
            &format!("devpit.app answered {}", answer.status()),
        ),
        Err(_) => devpit_core::reports::background("telegram", &"devpit.app did not answer"),
    }
}

/// devpit's bot, as devpit.app tells this account of it.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct There {
    telegram_bot: bool,
    bot: Option<String>,
    linked: bool,
}

/// What devpit.app says of its bot; nothing when not signed in, the hub is
/// off there, or it did not answer.
async fn there() -> Option<There> {
    let token = crate::account::token::read()?;
    let answer = crate::account::client()
        .ok()?
        .get(format!("{}/v1/hub/telegram", crate::account::origin()))
        .bearer_auth(token)
        .send()
        .await
        .ok()?;
    if !answer.status().is_success() {
        return None;
    }
    answer.json().await.ok()
}

/// The Telegram status, when no bot of the person's own is set up.
pub(crate) async fn status() -> Result<TelegramStatus, RpcError> {
    let there = there().await;
    if let Some(there) = &there {
        seen(there.linked);
    }
    let kept = read().unwrap_or_default();
    let now = devpit_core::reports::now() as i64;
    let waiting = !kept.linked && kept.code_until > now;
    let orchestrators =
        crate::off_main::blocking(|| Ok(crate::channel_talk::orchestrators())).await?;
    Ok(TelegramStatus {
        linked: kept.linked,
        bot: there
            .as_ref()
            .and_then(|there| there.bot.clone())
            .or((!kept.bot.is_empty()).then(|| kept.bot.clone())),
        code: kept.code.clone().filter(|_| waiting),
        link: kept.link.clone().filter(|_| waiting),
        privacy: kept.privacy(),
        orchestrator: kept.orchestrator.clone(),
        orchestrators,
        devpit_bot: there.is_some_and(|there| there.telegram_bot),
        through_devpit: kept.linked,
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Code {
    code: String,
    link: String,
    bot: String,
}

/// `telegram.use_devpit` — a single-use code from devpit.app, and the link
/// that sends it to devpit's bot.
pub(crate) async fn link() -> Result<TelegramStatus, RpcError> {
    let token = crate::account::token::read().ok_or_else(|| {
        RpcError::new(
            devpit_rpc::ErrorCode::Invalid,
            "sign in to devpit.app first",
        )
    })?;
    let answer = crate::account::client()?
        .post(format!("{}/v1/hub/telegram/link", crate::account::origin()))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|_| RpcError::internal("devpit.app did not answer"))?;
    if !answer.status().is_success() {
        #[derive(Deserialize)]
        struct Refused {
            message: String,
        }
        let why = answer
            .json::<Refused>()
            .await
            .map_or_else(|_| "devpit.app refused it".to_owned(), |said| said.message);
        return Err(RpcError::new(devpit_rpc::ErrorCode::Invalid, why));
    }
    let code: Code = answer
        .json()
        .await
        .map_err(|_| RpcError::internal("devpit.app's answer did not read"))?;
    let mut kept = read().unwrap_or_default();
    kept.linked = false;
    kept.bot = code.bot;
    kept.code = Some(code.code);
    kept.link = Some(code.link);
    kept.code_until = devpit_core::reports::now() as i64 + CODE_SECS;
    write(&kept)?;
    status().await
}

/// Forgets the chat, there and here.
pub(crate) async fn unlink() {
    if let (Some(token), Ok(client)) = (crate::account::token::read(), crate::account::client()) {
        let _ = client
            .delete(format!("{}/v1/hub/telegram", crate::account::origin()))
            .bearer_auth(token)
            .send()
            .await;
    }
    if let Some(path) = file() {
        let _ = std::fs::remove_file(path);
    }
}

/// How much the chat is told: names at most, through devpit.app.
pub(crate) fn privacy_set(privacy: Privacy) -> Result<(), RpcError> {
    if privacy == Privacy::Full {
        return Err(RpcError::new(
            devpit_rpc::ErrorCode::Invalid,
            "a whole answer goes only through your own bot; devpit's carries names at most",
        ));
    }
    let mut kept = read().unwrap_or_default();
    kept.privacy = Some(privacy);
    write(&kept)
}

pub(crate) fn orchestrator_set(project: Option<String>) -> Result<(), RpcError> {
    let mut kept = read().unwrap_or_default();
    kept.orchestrator = project;
    write(&kept)
}

#[cfg(test)]
#[path = "telegram_devpit_tests.rs"]
mod tests;
