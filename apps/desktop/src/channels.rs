//! What happens in devpit, told on the person's channels while they are away
//! from it: which events go where is `channel_rules`; this gathers them for a
//! moment, so five sessions ending at once are one message, and sends.
//!
//! Only when devpit's window is not in front and nothing is paused — the bell
//! and the island already say it to someone looking.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use devpit_core::preference;
use devpit_rpc::{ChannelEvent, ChannelInfo, ChannelRules, Channels, RpcError};

/// A button under a message, and what it does back on this machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Action {
    /// Send the draft waiting for a session, as the person.
    SendDraft {
        profile: String,
        session: String,
    },
    /// A reminder: done, or again in fifteen minutes.
    ReminderDone {
        card_id: String,
    },
    Snooze {
        card_id: String,
    },
}

/// One thing to tell: what it is, a line saying it, and what it offers.
#[derive(Debug, Clone)]
pub(crate) struct Told {
    pub event: ChannelEvent,
    /// With the session, card or project named.
    pub line: String,
    /// Without names, for a channel that says only that something happened.
    pub bare: String,
    pub actions: Vec<Action>,
}

fn queue() -> &'static Mutex<Vec<Told>> {
    static QUEUE: OnceLock<Mutex<Vec<Told>>> = OnceLock::new();
    QUEUE.get_or_init(Mutex::default)
}

static GATHERING: AtomicBool = AtomicBool::new(false);

/// The channels this machine can send on now.
fn connected() -> Vec<ChannelInfo> {
    crate::telegram::connected()
        .into_iter()
        .chain(crate::hub::connected())
        .collect()
}

fn ids(connected: &[ChannelInfo]) -> Vec<String> {
    connected.iter().map(|one| one.id.clone()).collect()
}

/// The rules the person set, or the defaults for what is connected.
pub(crate) fn rules() -> ChannelRules {
    let connected = ids(&connected());
    crate::projects::store()
        .ok()
        .and_then(|store| store.preference(preference::CHANNEL_RULES).ok().flatten())
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_else(|| crate::channel_rules::defaults(&connected))
}

/// Tells `told` on the channels its event goes to, after gathering a moment.
/// Nothing at all while devpit is paused or no channel is connected.
pub(crate) fn emit(told: Told) {
    if crate::pausing::paused() || connected().is_empty() {
        return;
    }
    if let Ok(mut held) = queue().lock() {
        held.push(told);
    }
    if GATHERING.swap(true, Ordering::SeqCst) {
        return;
    }
    let wait = Duration::from_secs(u64::from(rules().group_seconds.clamp(5, 600)));
    std::thread::spawn(move || {
        std::thread::sleep(wait);
        GATHERING.store(false, Ordering::SeqCst);
        let batch = queue()
            .lock()
            .map(|mut held| std::mem::take(&mut *held))
            .unwrap_or_default();
        deliver(&batch);
    });
}

/// Sends a gathered batch: per channel, one message, with buttons only when
/// it is about one thing.
fn deliver(batch: &[Told]) {
    // Someone looking at devpit has the bell and the island.
    if crate::telegram::app()
        .get()
        .is_some_and(crate::island_notify::in_front)
    {
        return;
    }
    let rules = rules();
    let channels = connected();
    let offset = rules.quiet.as_ref().map_or(0, |hours| hours.offset_minutes);
    let (weekday, minute) = crate::channel_rules::local(devpit_core::reports::now() as i64, offset);
    for channel in &channels {
        let mine: Vec<&Told> = batch
            .iter()
            .filter(|one| {
                crate::channel_rules::routed(&rules, one.event, &ids(&channels), weekday, minute)
                    .contains(&channel.id)
            })
            .collect();
        let titles = if channel.id == crate::hub::ID {
            crate::hub::titles()
        } else {
            crate::telegram::titles()
        };
        let lines: Vec<String> = mine
            .iter()
            .map(|one| {
                if titles {
                    one.line.clone()
                } else {
                    one.bare.clone()
                }
            })
            .collect();
        let Some(text) = crate::channel_rules::grouped(&lines) else {
            continue;
        };
        let actions = match mine.as_slice() {
            [one] => one.actions.clone(),
            _ => Vec::new(),
        };
        match channel.id.as_str() {
            crate::telegram::ID => crate::telegram::send(&text, &actions),
            crate::hub::ID => crate::hub::send(&text, &actions),
            _ => {}
        }
    }
}

/// `channels.read` — what is connected, and the rules.
#[tauri::command]
#[specta::specta]
pub async fn channels_read() -> Result<Channels, RpcError> {
    crate::off_main::blocking(|| {
        Ok(Channels {
            connected: connected(),
            rules: rules(),
        })
    })
    .await
}

/// `channels.rules_set` — which events go where, quiet hours, grouping.
#[tauri::command]
#[specta::specta]
pub async fn channels_rules_set(rules: ChannelRules) -> Result<Channels, RpcError> {
    crate::off_main::blocking(move || {
        let text =
            serde_json::to_string(&rules).map_err(|err| RpcError::internal(err.to_string()))?;
        crate::projects::store()?.set_preference(preference::CHANNEL_RULES, &text)?;
        Ok(Channels {
            connected: connected(),
            rules: crate::channels::rules(),
        })
    })
    .await
}
