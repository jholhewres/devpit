//! A device that proved the pairing secret waits here for the person at the
//! machine to allow it: a QR code seen over a shoulder, or a link sent on,
//! does not pair a device by itself.

use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;
use std::time::Duration;

use devpit_rpc::RemoteAsking;

/// How long a device waits for the person before it is refused.
pub(crate) const ASK_FOR: Duration = Duration::from_secs(120);

struct Asking {
    id: String,
    name: String,
    answer: Sender<bool>,
}

static ASKING: Mutex<Vec<Asking>> = Mutex::new(Vec::new());

/// Whether the person allows the device `name`: asked on the machine's screen
/// and waited for; refused when they say no, or nothing within [`ASK_FOR`].
pub(crate) fn allowed(app: &tauri::AppHandle, name: &str) -> bool {
    let (id, answer) = ask(name);
    crate::remote::changed(app);
    let said = waited(&answer, ASK_FOR);
    withdraw(&id);
    crate::remote::changed(app);
    said
}

/// A new question about `name`, and where its answer comes.
pub(crate) fn ask(name: &str) -> (String, Receiver<bool>) {
    let id = ulid::Ulid::generate().to_string();
    let (answer, heard) = channel();
    if let Ok(mut asking) = ASKING.lock() {
        asking.push(Asking {
            id: id.clone(),
            name: name.to_owned(),
            answer,
        });
    }
    (id, heard)
}

/// The person's answer, or no once `within` has passed.
pub(crate) fn waited(answer: &Receiver<bool>, within: Duration) -> bool {
    answer.recv_timeout(within).unwrap_or(false)
}

/// The person's answer to question `id`; false when nothing asks it now.
pub(crate) fn answer(id: &str, allow: bool) -> bool {
    let Ok(mut asking) = ASKING.lock() else {
        return false;
    };
    let Some(at) = asking.iter().position(|one| one.id == id) else {
        return false;
    };
    asking.remove(at).answer.send(allow).is_ok()
}

fn withdraw(id: &str) {
    if let Ok(mut asking) = ASKING.lock() {
        asking.retain(|one| one.id != id);
    }
}

/// The devices waiting on the person now, as the settings show them.
pub(crate) fn asking() -> Vec<RemoteAsking> {
    ASKING
        .lock()
        .map(|asking| {
            asking
                .iter()
                .map(|one| RemoteAsking {
                    id: one.id.clone(),
                    name: one.name.clone(),
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_device_is_paired_only_when_the_person_says_yes() {
        let (id, heard) = ask("Pixel");
        assert!(asking()
            .iter()
            .any(|one| one.id == id && one.name == "Pixel"));
        assert!(answer(&id, true));
        assert!(waited(&heard, Duration::from_millis(10)));
        assert!(!asking().iter().any(|one| one.id == id), "asked twice");
        assert!(!answer(&id, true), "an answer counts once");

        let (id, heard) = ask("Stranger");
        assert!(answer(&id, false));
        assert!(!waited(&heard, Duration::from_millis(10)));
    }

    #[test]
    fn silence_is_a_no() {
        let (id, heard) = ask("Unattended");
        assert!(!waited(&heard, Duration::from_millis(20)));
        withdraw(&id);
        assert!(!answer(&id, true), "a question nobody waits on any more");
    }
}
