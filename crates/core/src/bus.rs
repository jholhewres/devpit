//! Topic-scoped event bus over `tokio::sync::broadcast`.
//!
//! It exists to enforce one rule: the UI never derives state from a command
//! response. It sends the command, gets `ok`, and waits for the event.
//!
//! That kills the class of bug where two windows diverge: a backend that
//! answers correctly while the sidebar draws something else, because the
//! state that mattered lived in the interface.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::sync::broadcast;

/// How far behind a slow subscriber may fall before it loses the oldest
/// events. Dropping is deliberate: a slow client that blocked the producer
/// would stall the whole core over one minimised window. It gets `Lagged` and
/// answers with a resync.
const BACKLOG: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub topic: String,
    pub scope_id: Option<String>,
    pub severity: Severity,
    pub payload: String,
}

/// The emitter decides the colour, not the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    /// Including an agent working. Normal motion is not an alarm.
    Neutral,
    /// A decision of yours blocks progress.
    Attention,
    Failure,
    /// A claim backed by a stored artifact. Never emitted by activity.
    Verified,
}

#[derive(Clone, Default)]
pub struct Bus {
    topics: Arc<Mutex<HashMap<String, broadcast::Sender<Event>>>>,
}

impl Bus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn subscribe(&self, topic: &str) -> broadcast::Receiver<Event> {
        self.sender(topic).subscribe()
    }

    /// No listener is not an error: it is the normal case for a screen that
    /// has not been opened yet.
    pub fn publish(&self, event: Event) {
        let _ = self.sender(&event.topic).send(event);
    }

    fn sender(&self, topic: &str) -> broadcast::Sender<Event> {
        let mut topics = self.topics.lock().expect("poisoned bus");
        topics
            .entry(topic.to_owned())
            .or_insert_with(|| broadcast::channel(BACKLOG).0)
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(topic: &str) -> Event {
        Event {
            topic: topic.to_owned(),
            scope_id: None,
            severity: Severity::Neutral,
            payload: "{}".to_owned(),
        }
    }

    #[tokio::test]
    async fn a_subscriber_receives_its_own_topic() {
        let bus = Bus::new();
        let mut rx = bus.subscribe("session.state");

        bus.publish(event("session.state"));

        assert_eq!(rx.recv().await.expect("receive"), event("session.state"));
    }

    #[tokio::test]
    async fn topics_do_not_leak_into_each_other() {
        let bus = Bus::new();
        let mut rx = bus.subscribe("fs.changed");

        bus.publish(event("git.changed"));

        assert!(rx.try_recv().is_err(), "event leaked across topics");
    }

    #[test]
    fn publishing_with_no_subscriber_does_not_panic() {
        Bus::new().publish(event("nobody.listens"));
    }
}
