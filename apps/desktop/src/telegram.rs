//! Telegram, through the person's own bot: devpit's notices sent from this
//! machine, and their buttons answered here. No devpit.app in between.
//!
//! The token is kept in `telegram.json` under devpit's root, readable by the
//! person alone, and never shown, logged or put in an error — an HTTP error
//! names its URL, and a Bot API URL holds the token, so none is passed on.
//! Linking: devpit shows a code, the person sends `/start <code>` to their
//! bot, and that chat is the one devpit talks to and listens to; any other
//! chat is ignored.

use std::sync::OnceLock;
use std::time::Duration;

use devpit_rpc::{ChannelInfo, RpcError, TelegramStatus};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::AppHandle;

pub(crate) const ID: &str = "telegram";
const API: &str = "https://api.telegram.org";
/// How long a linking code holds.
const CODE_SECS: i64 = 10 * 60;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct Kept {
    token: String,
    bot: String,
    chat_id: Option<i64>,
    code: Option<String>,
    code_until: i64,
    titles: bool,
}

fn file() -> Option<std::path::PathBuf> {
    devpit_core::Store::root()
        .ok()
        .map(|root| root.join("telegram.json"))
}

fn read() -> Option<Kept> {
    serde_json::from_str(&std::fs::read_to_string(file()?).ok()?).ok()
}

fn write(kept: &Kept) -> Result<(), RpcError> {
    let path = file().ok_or_else(|| RpcError::internal("no home directory to write to"))?;
    let text = serde_json::to_vec(kept).map_err(|err| RpcError::internal(err.to_string()))?;
    devpit_core::home::write_private(&path, &text)
        .map_err(|err| RpcError::internal(err.to_string()))
}

pub(crate) fn app() -> &'static OnceLock<AppHandle> {
    static APP: OnceLock<AppHandle> = OnceLock::new();
    &APP
}

/// The channel, when a chat is linked.
pub(crate) fn connected() -> Option<ChannelInfo> {
    let kept = read()?;
    kept.chat_id.map(|_| ChannelInfo {
        id: ID.to_owned(),
        label: format!("Telegram, @{}", kept.bot),
    })
}

/// Whether messages say what they are about, or only that something happened.
pub(crate) fn titles() -> bool {
    read().is_some_and(|kept| kept.titles)
}

/// A call to the Bot API. Any failure is one sentence with no URL in it.
async fn call(token: &str, method: &str, body: Value) -> Result<Value, String> {
    let answer = crate::account::client()
        .map_err(|err| err.message)?
        .post(format!("{API}/bot{token}/{method}"))
        .json(&body)
        .timeout(Duration::from_secs(60))
        .send()
        .await
        .map_err(|_| "Telegram did not answer".to_owned())?;
    let said: Value = answer
        .json()
        .await
        .map_err(|_| "Telegram's answer did not read".to_owned())?;
    if said["ok"] != true {
        return Err(said["description"]
            .as_str()
            .unwrap_or("Telegram refused it")
            .to_owned());
    }
    Ok(said["result"].clone())
}

/// Sends `text` to the linked chat, with a button per action.
pub(crate) fn send(text: &str, offered: &[crate::channels::Action]) {
    let Some(kept) = read() else { return };
    let Some(chat) = kept.chat_id else { return };
    let row: Vec<Value> = offered
        .iter()
        .map(|action| {
            let key = crate::channel_actions::remember(action);
            json!({ "text": crate::channel_actions::label(action), "callback_data": format!("a:{key}") })
        })
        .collect();
    let mut body = json!({ "chat_id": chat, "text": text });
    if !row.is_empty() {
        body["reply_markup"] = json!({ "inline_keyboard": [row] });
    }
    let token = kept.token;
    tauri::async_runtime::spawn(async move {
        if let Err(why) = call(&token, "sendMessage", body).await {
            devpit_core::reports::background("telegram", &why);
        }
    });
}

/// What a code the person sent links, if it is the one devpit showed.
pub(crate) fn links(kept: &Kept, said: &str, now: i64) -> bool {
    let Some(code) = kept.code.as_deref() else {
        return false;
    };
    let said = said.trim();
    let said = said.strip_prefix("/start").map(str::trim).unwrap_or(said);
    now < kept.code_until && said.eq_ignore_ascii_case(code)
}

/// One update from the bot: a linking code, or a button pressed in the linked chat.
async fn heard(update: &Value) {
    let Some(mut kept) = read() else { return };
    let now = devpit_core::reports::now() as i64;
    if let Some(message) = update.get("message") {
        let chat = message["chat"]["id"].as_i64();
        let private = message["chat"]["type"] == "private";
        let text = message["text"].as_str().unwrap_or_default();
        if private && links(&kept, text, now) {
            kept.chat_id = chat;
            kept.code = None;
            if write(&kept).is_ok() {
                let _ = call(&kept.token, "sendMessage", json!({ "chat_id": chat, "text": "Linked: devpit tells you here what waits on you." })).await;
            }
        }
        return;
    }
    let Some(pressed) = update.get("callback_query") else {
        return;
    };
    let from_linked =
        pressed["message"]["chat"]["id"].as_i64() == kept.chat_id && kept.chat_id.is_some();
    let key = pressed["data"]
        .as_str()
        .and_then(|data| data.strip_prefix("a:"))
        .unwrap_or_default();
    let action = from_linked
        .then(|| crate::channel_actions::take(key))
        .flatten();
    let said = action.as_ref().map_or_else(
        || "That button has expired.".to_owned(),
        |action| crate::channel_actions::act(action, "Telegram"),
    );
    let _ = call(
        &kept.token,
        "answerCallbackQuery",
        json!({ "callback_query_id": pressed["id"], "text": said }),
    )
    .await;
}

static LISTENING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Listens to the bot while one is set up: long polling, from this machine.
pub(crate) fn listen(app: &AppHandle) {
    let _ = self::app().set(app.clone());
    if read().is_none() || LISTENING.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    tauri::async_runtime::spawn(async {
        let mut offset = 0i64;
        while let Some(kept) = read() {
            match call(&kept.token, "getUpdates", json!({ "offset": offset, "timeout": 50, "allowed_updates": ["message", "callback_query"] })).await {
                Ok(Value::Array(updates)) => {
                    for update in &updates {
                        offset = offset.max(update["update_id"].as_i64().unwrap_or(0) + 1);
                        heard(update).await;
                    }
                }
                _ => tokio::time::sleep(Duration::from_secs(30)).await,
            }
        }
        LISTENING.store(false, std::sync::atomic::Ordering::SeqCst);
    });
}

fn status(kept: Option<&Kept>) -> TelegramStatus {
    TelegramStatus {
        linked: kept.is_some_and(|kept| kept.chat_id.is_some()),
        bot: kept.map(|kept| kept.bot.clone()),
        code: kept.and_then(|kept| kept.code.clone()),
        link: kept.and_then(|kept| {
            kept.code
                .as_ref()
                .map(|code| format!("https://t.me/{}?start={code}", kept.bot))
        }),
        titles: kept.is_some_and(|kept| kept.titles),
    }
}

/// `telegram.status`
#[tauri::command]
#[specta::specta]
pub async fn telegram_status() -> Result<TelegramStatus, RpcError> {
    Ok(status(read().as_ref()))
}

/// `telegram.link` — takes the person's bot token, checks it, and offers a code
/// to send to the bot from the chat devpit should use.
#[tauri::command]
#[specta::specta]
pub async fn telegram_link(app: AppHandle, token: String) -> Result<TelegramStatus, RpcError> {
    let token = token.trim().to_owned();
    if token.is_empty()
        || token.len() > 100
        || !token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '_' | '-'))
    {
        return Err(RpcError::new(
            devpit_rpc::ErrorCode::Invalid,
            "that is not a bot token from @BotFather",
        ));
    }
    let me = call(&token, "getMe", json!({})).await.map_err(|_| {
        RpcError::new(
            devpit_rpc::ErrorCode::Invalid,
            "Telegram does not know that token",
        )
    })?;
    let code: String = ulid::Ulid::generate()
        .to_string()
        .chars()
        .rev()
        .take(6)
        .collect();
    let kept = Kept {
        token,
        bot: me["username"].as_str().unwrap_or_default().to_owned(),
        chat_id: None,
        code: Some(code),
        code_until: devpit_core::reports::now() as i64 + CODE_SECS,
        titles: read().is_some_and(|was| was.titles),
    };
    write(&kept)?;
    listen(&app);
    Ok(status(Some(&kept)))
}

/// `telegram.titles_set` — whether messages name what they are about.
#[tauri::command]
#[specta::specta]
pub async fn telegram_titles_set(titles: bool) -> Result<TelegramStatus, RpcError> {
    let mut kept =
        read().ok_or_else(|| RpcError::new(devpit_rpc::ErrorCode::NotFound, "no bot is set up"))?;
    kept.titles = titles;
    write(&kept)?;
    Ok(status(Some(&kept)))
}

/// `telegram.unlink` — forgets the bot and the chat.
#[tauri::command]
#[specta::specta]
pub async fn telegram_unlink() -> Result<TelegramStatus, RpcError> {
    if let Some(path) = file() {
        let _ = std::fs::remove_file(path);
    }
    Ok(status(None))
}

#[cfg(test)]
#[path = "telegram_tests.rs"]
mod tests;
