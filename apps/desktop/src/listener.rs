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
use tauri::{AppHandle, Emitter, Manager};

use crate::asking::{decision, Asking};
use crate::card_activity::{pane_word, state_of_event};
use crate::card_route::{card_of_leaf, notice_for, Ring};
use crate::happening::{agent_said, session_said, subagent_said};
use crate::post::read_request;
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
    let endpoint = devpit_agentcli::endpoint_file(root);
    if let Some(parent) = endpoint.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(err) = std::fs::write(&endpoint, format!("http://{address}/hook")) {
        eprintln!("could not publish the hook endpoint: {err}");
        return;
    }

    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let app = app.clone();
            // One thread per post, and they are short: a hook that has to wait
            // for the one before it is a hook holding up the agent that sent it.
            std::thread::spawn(move || serve(app, stream));
        }
    });
}

fn serve(app: AppHandle, mut stream: TcpStream) {
    let Some(posted) = read_request(&mut stream) else {
        let _ = stream.write_all(b"HTTP/1.1 400 Bad Request\r\ncontent-length: 0\r\n\r\n");
        return;
    };
    let body = posted.body;

    // A tool this session wants to be asked about holds the connection until
    // a person answers. Everything else is answered at once and empty, which
    // leaves the CLI's own permission mode in charge — a board step running
    // where nobody is watching must never wait on a window.
    let held = question_in(&body).filter(|question| {
        app.try_state::<Asking>()
            .map(|asking| asking.asks(&question.session_id))
            .unwrap_or(false)
    });

    if let Some(question) = held {
        if let Some(happening) = read_hook(&body) {
            let _ = app.emit("agent:happening", describe(&happening));
            heard(&app, posted.pane.as_deref(), &happening);
        }
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
    if let Some(happening) = read_hook(&body) {
        let _ = app.emit("agent:happening", describe(&happening));
        heard(&app, posted.pane.as_deref(), &happening);
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
fn heard(sink: &impl HookSink, pane: Option<&str>, happening: &Happening) {
    let Some(pane) = pane else {
        return;
    };
    let state = pane_word(state_of_event(&happening.event));
    // One Store for the event. Resolved before `remember`, which forgets the
    // pane's agent on SessionEnded — the end has to reach the card too.
    let store = Store::open_default().ok();
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

    // Only `waiting` reaches the bell. An agent that is working is an agent
    // you can watch; one that has stopped and is waiting for a person is the
    // reason somebody left the window and the reason to call them back. The
    // other two would be a bell that rings through every turn.
    if let (Some(ring), Some(store)) = (notice_for(route.as_ref(), state), &store) {
        sink.ring(store, ring, pane);
    }
}

/// Where what a hook says goes: the window, and the bell.
///
/// A trait rather than the `AppHandle`, so the listener's core runs without a
/// window.
pub(crate) trait HookSink {
    fn to_window<P: serde::Serialize + Clone>(&self, channel: &str, payload: P);
    /// A pane worth coming back to, with the Store the event already opened.
    fn ring(&self, store: &Store, ring: Ring<'_>, pane: &str);
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
        Event::Using { tool } => format!("running {tool}"),
        Event::Used { tool } => format!("finished {tool}"),
        Event::Stopped { said } => said
            .clone()
            .unwrap_or_else(|| "finished the turn".to_owned()),
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
