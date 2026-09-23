//! The other end of the hooks: a loopback listener the agent posts to.
//!
//! Small on purpose. It answers one route, on 127.0.0.1, on a port the OS
//! picks, and writes that address to a file the hook reads on every invocation
//! — so a session that outlived a restart finds the new port instead of posting
//! into a dead one.
//!
//! Loopback and nothing else. This receives a payload and turns it into
//! something the window draws; binding it anywhere reachable would be a way in
//! to a process that runs terminals.

use std::io::Write;
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::path::Path;

use devpit_agentcli::{read_hook, Event, Happening};
use devpit_core::Store;
use devpit_rpc::SessionKind;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

use crate::asking::{decision, Asking};
use crate::card_activity::{hear, next_seq, pane_word, state_of_event, Activities, Key, Place};
use crate::card_route::{card_of_leaf, card_of_session, notice_for, Ring};
use crate::happening::{agent_said, session_said, subagent_said};
use crate::post::{read_request, Posted};
use crate::question::question_in;

/// Starts listening, and writes the address where the hook will look for it.
///
/// Failure is not fatal: hooks are how the board hears about work as it
/// happens, and without them it still polls. A window that refuses to open
/// because a port was busy would be worse than one that is a little less live.
pub fn start(app: AppHandle, root: &Path) {
    let listener = match TcpListener::bind((Ipv4Addr::LOCALHOST, 0)) {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!("no hook listener, the board will poll instead: {err}");
            return;
        }
    };

    let Ok(address) = listener.local_addr() else {
        return;
    };
    // Where one run of the app begins in the trace: sequence numbers start
    // again with every process, and a reader pairing them has to know that.
    trace("listening");
    let endpoint = devpit_agentcli::endpoint_file(root);
    if let Some(parent) = endpoint.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    // The secret before the address, and for the same reason the address is
    // written at all: a hook that found a port to post to but no secret to
    // carry would post and, from the next story on, be refused.
    let Some(secret) = fresh_secret() else {
        eprintln!("no hook listener: the system gave out no randomness");
        return;
    };
    if let Err(err) = devpit_core::home::write_private(
        &devpit_agentcli::auth_file(root),
        format!("{}: {secret}\n", devpit_agentcli::HOOK_HEADER).as_bytes(),
    ) {
        eprintln!("could not write the hook secret: {err}");
        return;
    }
    let _ = SECRET.set(secret);
    // Private: the port is what a post has to know, so a file anyone can read
    // is an invitation to post as the agent.
    if let Err(err) =
        devpit_core::home::write_private(&endpoint, format!("http://{address}/hook").as_bytes())
    {
        eprintln!("could not publish the hook endpoint: {err}");
        return;
    }

    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let app = app.clone();
            // Stamped here, in the order posts arrived: each is served on its
            // own thread, and threads finish in any order.
            let seq = next_seq();
            // One thread per post, and they are short: a hook that has to wait
            // for the one before it is a hook holding up the agent that sent it.
            std::thread::spawn(move || serve(app, stream, seq));
        }
    });
}

/// This run's secret, kept for the door to compare against.
static SECRET: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// The hook secret this run of the app is using, if it has one.
///
/// Read by `kept_out`, which takes it back out of a command's log before the
/// log is stored: a hook that fails prints the request it sent.
pub(crate) fn secret_now() -> Option<String> {
    SECRET.get().cloned()
}

/// Whether a post showed this run's secret.
///
/// Compared to the end even once it is known to differ: a comparison that
/// stops at the first wrong byte says, in how long it took, how much of the
/// secret the caller already has.
///
/// What this defends against: another **user** on the machine, who can see the
/// port but not read a file at 0600. Not against code running as you — that
/// code can read the secret as easily as devpit can.
pub(crate) fn authorized(carried: Option<&str>, secret: &str) -> bool {
    let Some(carried) = carried else {
        return false;
    };
    if carried.len() != secret.len() {
        return false;
    }
    carried
        .bytes()
        .zip(secret.bytes())
        .fold(0u8, |differs, (mine, theirs)| differs | (mine ^ theirs))
        == 0
}

/// The secret this run's hooks carry, thirty-two bytes from the system's own
/// randomness.
///
/// Not `fresh_session_id` and not a ULID: those are a timestamp and a counter,
/// and a secret anything can recompute from the moment the app started is not
/// one. `None` when the system refuses, which is a reason not to listen at all
/// rather than to listen without a door.
fn fresh_secret() -> Option<String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).ok()?;
    Some(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn serve(app: AppHandle, mut stream: TcpStream, seq: u64) {
    trace(&format!("post seq={seq}"));
    let Some(posted) = read_request(&mut stream) else {
        trace(&format!("post seq={seq} bad request"));
        let _ = stream.write_all(b"HTTP/1.1 400 Bad Request\r\ncontent-length: 0\r\n\r\n");
        return;
    };
    // Refused before anything is read out of the payload. Counted, never
    // logged with what it carried: a refusal written to a log is the guess
    // written to a log.
    if let Some(secret) = SECRET.get() {
        if !authorized(posted.secret.as_deref(), secret) {
            trace(&format!("post seq={seq} refused"));
            let _ = stream.write_all(b"HTTP/1.1 401 Unauthorized\r\ncontent-length: 0\r\n\r\n");
            return;
        }
    }
    let body = &posted.body;

    // A question from an agent, not a report about one: answered with what it
    // asked for, and nothing about it reaches the hook path below.
    if posted.agent {
        trace(&format!("post seq={seq} agent"));
        reply(&mut stream, &crate::agent_api::answer(Some(&app), body));
        return;
    }

    // A tool this session wants to be asked about holds the connection until
    // a person answers. Everything else is answered at once and empty, which
    // leaves the CLI's own permission mode in charge — a board step running
    // where nobody is watching must never wait on a window.
    let held = question_in(body).filter(|question| {
        crate::question::changes_something(&question.tool)
            && app
                .try_state::<Asking>()
                .map(|asking| asking.asks(&question.session_id))
                .unwrap_or(false)
    });

    if let Some(question) = held {
        let settled = hear_post(&app, &posted, seq);
        trace(&format!("post seq={seq} {settled}"));
        let asking = app.state::<Asking>();
        let hear = asking.opened(&question.id);
        let _ = app.emit("permission:asked", &question);
        let said = decision(asking.wait(&question.id, hear));
        reply(&mut stream, &said);
        return;
    }

    // Answered before the payload is looked at. The agent is waiting on this
    // reply with a short budget, and nothing it says changes what we reply
    // with.
    reply(&mut stream, "");
    let settled = hear_post(&app, &posted, seq);
    trace(&format!("post seq={seq} {settled}"));
}

/// What a post becomes once it has been answered: the listener's core, apart
/// from the window.
///
/// Answers how it settled, in a word the trace prints: a post let in and never
/// accounted for is the one a test cannot see missing.
fn hear_post(sink: &impl HookSink, posted: &Posted, seq: u64) -> &'static str {
    match read_hook(&posted.body) {
        // An event this app does not read: settled, by being left alone.
        None => "ignored",
        Some(happening) => {
            sink.to_window("agent:happening", describe(&happening));
            heard(sink, posted.pane.as_deref(), &happening, seq)
        }
    }
}

/// Relays an agent's own report to the pane it is running in.
///
/// Only when the hook said which pane, which it does whenever the agent was
/// started inside one of our terminals. A headless turn belongs to a card and
/// has no pane to tell; it takes the `agent:happening` path above and this
/// leaves it alone.
///
/// This is the difference between knowing an agent is *open* and knowing what
/// it is *doing*. The first is asked of the process table on a timer; the
/// second only the agent can say, and it says it here.
fn heard(
    sink: &impl HookSink,
    pane: Option<&str>,
    happening: &Happening,
    seq: u64,
) -> &'static str {
    let Some(pane) = pane else {
        return heard_without_pane(sink, happening, seq);
    };
    let doing = state_of_event(&happening.event);
    let state = pane_word(doing);
    // One Store for the event. Resolved before `remember`, which forgets the
    // pane's agent on SessionEnded — the end has to reach the card too.
    let store = sink.store();
    let route = store.as_ref().and_then(|store| card_of_leaf(store, pane));
    if let Some(store) = &store {
        crate::restoring::remember(store, pane, happening);
    }
    if let Some(state) = state {
        sink.to_window("terminal:happening", agent_said(pane, state));
    }
    if let Some(session) = session_said(pane, happening) {
        sink.to_window("terminal:happening", session);
    }
    if let Some(subagent) = subagent_said(pane, &happening.event) {
        sink.to_window("terminal:happening", subagent);
    }
    let mut settled = match (&route, doing) {
        (_, None) => "no state",
        (None, _) if store.is_none() => "no store",
        (None, _) => "no card",
        (Some(_), Some(_)) => "unchanged",
    };
    if let (Some(route), Some(doing)) = (&route, doing) {
        let key = Key {
            card_id: route.card_id.clone(),
            kind: SessionKind::Pane,
            reference: pane.to_owned(),
        };
        let place = Place {
            tab_id: Some(route.tab_id.clone()),
            leaf_id: Some(pane.to_owned()),
        };
        let told = sink
            .activities()
            .lock()
            .ok()
            .and_then(|mut activities| hear(&mut activities, key, seq, doing, place));
        if let Some(happening) = told {
            trace(&format!("emit card:happening seq={seq}"));
            sink.to_window("card:happening", happening);
            settled = "emitted";
        }
    }

    // Only `waiting` reaches the bell. An agent that is working is an agent
    // you can watch; one that has stopped and is waiting for a person is the
    // reason somebody left the window and the reason to call them back. The
    // other two would be a bell that rings through every turn.
    if let (Some(ring), Some(store)) = (notice_for(route.as_ref(), state), &store) {
        sink.ring(store, ring, pane);
    }
    settled
}

/// A hook from a session with no pane — a run's turn, or a session a step
/// started in the background — reaching the card that holds its id.
///
/// Heard like a pane is and combined when read: a background session's word
/// stands only while the CLI still lists it (see `background_state`).
fn heard_without_pane(sink: &impl HookSink, happening: &Happening, seq: u64) -> &'static str {
    let Some(doing) = state_of_event(&happening.event) else {
        return "no state";
    };
    let Some(store) = sink.store() else {
        return "no store";
    };
    let Some((card_id, kind)) = card_of_session(&store, &happening.session_id) else {
        return "no card";
    };
    let key = Key {
        card_id,
        kind,
        reference: happening.session_id.clone(),
    };
    let told = sink
        .activities()
        .lock()
        .ok()
        .and_then(|mut activities| hear(&mut activities, key, seq, doing, Place::default()));
    match told {
        Some(happening) => {
            trace(&format!("emit card:happening seq={seq}"));
            sink.to_window("card:happening", happening);
            "emitted"
        }
        None => "unchanged",
    }
}

/// A timestamped line on stderr, only with `DEVPIT_TRACE_HOOKS` set: how long a
/// hook's post takes to reach the window is measured from these, by a person
/// running the app, and nobody else pays for the lines.
fn trace(what: &str) {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if !*ON.get_or_init(|| std::env::var_os("DEVPIT_TRACE_HOOKS").is_some()) {
        return;
    }
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_millis())
        .unwrap_or_default();
    eprintln!("devpit-trace {millis} {what}");
}

/// Where what a hook says goes: the window, and the bell.
///
/// A trait rather than the `AppHandle`, so the listener's core runs without a
/// window.
pub(crate) trait HookSink {
    fn to_window<P: serde::Serialize + Clone>(&self, channel: &str, payload: P);
    /// A pane worth coming back to, with the Store the event already opened.
    fn ring(&self, store: &Store, ring: Ring<'_>, pane: &str);
    /// The Store for one event, or nothing when it cannot be opened.
    fn store(&self) -> Option<Store>;
    /// What has been heard about cards so far.
    fn activities(&self) -> &Mutex<Activities>;
}

impl HookSink for AppHandle {
    fn to_window<P: serde::Serialize + Clone>(&self, channel: &str, payload: P) {
        let _ = self.emit(channel, payload);
    }

    fn ring(&self, store: &Store, ring: Ring<'_>, pane: &str) {
        crate::notices::ring_in(
            store,
            self,
            ring.project_id,
            crate::notices::kind::AGENT,
            "An agent is waiting on you",
            Some(pane),
            ring.card_id,
        );
    }

    fn store(&self) -> Option<Store> {
        Store::open_default().ok()
    }

    fn activities(&self) -> &Mutex<Activities> {
        crate::card_activity::registry()
    }
}

fn reply(stream: &mut TcpStream, body: &str) {
    let _ = stream.write_all(
        format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
            body.len()
        )
        .as_bytes(),
    );
    let _ = stream.flush();
}

/// What the window is told, in the words it draws.
fn describe(happening: &Happening) -> (String, String) {
    let said = match &happening.event {
        Event::Prompted => "took a prompt".to_owned(),
        Event::Using { tool } => format!("running {tool}"),
        Event::Used { tool } => format!("finished {tool}"),
        Event::Stopped { said } => said
            .clone()
            .unwrap_or_else(|| "finished the turn".to_owned()),
        Event::Failed { error: Some(error) } => format!("stopped on an error: {error}"),
        Event::Failed { error: None } => "stopped on an error".to_owned(),
        Event::SubagentStarted {
            kind: Some(kind), ..
        } => format!("started a {kind} subagent"),
        Event::SubagentStarted { kind: None, .. } => "started a subagent".to_owned(),
        Event::Delegated {
            description: Some(description),
            ..
        } => format!("handed off: {description}"),
        Event::Delegated { .. } => "handed work to a subagent".to_owned(),
        Event::SubagentDone { .. } => "a subagent finished".to_owned(),
        Event::Waiting => "waiting on you".to_owned(),
        Event::SessionStarted => "started a session".to_owned(),
        Event::SessionEnded { .. } => "ended the session".to_owned(),
    };
    (happening.session_id.clone(), said)
}

#[cfg(test)]
#[path = "listener_tests.rs"]
mod tests;
