//! Which machines sync, as the settings list them: each writes its own
//! record — its name and, to the hour, when it last synced — and reads the
//! others'. Sealed like the rest, so devpit.app does not learn the names.

use crate::doc::Doc;
use crate::space::{Here, Namespace, Seen};

pub const NAME: &str = "name";
pub const SEEN: &str = "seen";
/// When it last synced is written to the hour: a round every few minutes
/// would otherwise send a new version every time.
const HOUR: u64 = 60 * 60;

/// This machine, among the account's.
pub struct Machines<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub now_ms: u64,
}

impl Namespace for Machines<'_> {
    fn name(&self) -> &'static str {
        "machines"
    }

    fn here(&self) -> Result<Here, String> {
        let hour = self.now_ms / 1000 / HOUR * HOUR;
        let name: String = self
            .name
            .chars()
            .filter(|c| !c.is_control())
            .take(80)
            .collect();
        Ok(Here::from([(
            self.id.to_owned(),
            [
                (NAME.to_owned(), Seen::of(Some(name), None)),
                (SEEN.to_owned(), Seen::of(Some(hour.to_string()), None)),
            ]
            .into(),
        )]))
    }

    /// Only this machine's own record is here, and it is read, not written.
    fn write(&self, _: &str, _: &str, _: Option<&str>) -> Result<(), String> {
        Ok(())
    }
}

/// A machine as the settings show it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Machine {
    pub id: String,
    pub name: String,
    /// Seconds since the epoch, to the hour.
    pub seen: Option<u64>,
}

/// The machines a mirror of the namespace knows, most recently seen first.
pub fn listed(mirror: Option<&Doc>) -> Vec<Machine> {
    let mut machines: Vec<Machine> = mirror
        .map(|doc| {
            doc.records
                .iter()
                .map(|(id, fields)| Machine {
                    id: id.clone(),
                    name: fields
                        .get(NAME)
                        .and_then(|cell| cell.value.clone())
                        .unwrap_or_else(|| "a machine".to_owned()),
                    seen: fields
                        .get(SEEN)
                        .and_then(|cell| cell.value.as_deref())
                        .and_then(|seen| seen.parse().ok()),
                })
                .collect()
        })
        .unwrap_or_default();
    machines.sort_by(|a, b| b.seen.cmp(&a.seen).then(a.id.cmp(&b.id)));
    machines
}
