//! Reaching this machine from another device: what the viewer and the host
//! say to each other, and what the settings show about it.
//!
//! The viewer is a page this machine serves over the person's tailnet. It
//! speaks JSON over one WebSocket, a message at a time, tagged by `t`. What
//! it may do is decided here, on the machine, per paired device: every
//! device sees; typing and answering are granted one by one.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::{Board, Conversation, Conversations};

/// What a viewer asks.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(tag = "t", rename_all = "camelCase")]
pub enum RemoteIn {
    /// The first message: the device's token, which only its pairing gave it.
    Hello {
        token: String,
    },
    Projects,
    /// Watch a terminal, sized for the viewer's screen. The desk's own size
    /// does not change.
    PaneOpen {
        project: String,
        pane: String,
        cols: u16,
        rows: u16,
    },
    /// Keys, as bytes in base64. Only a device allowed to type.
    PaneInput {
        pane: String,
        b64: String,
    },
    /// A line written and sent, as if pasted and Enter pressed.
    PanePaste {
        pane: String,
        text: String,
    },
    PaneClose {
        pane: String,
    },
    Board {
        project: String,
    },
    /// Into a lane without a step. Only a device allowed to type.
    CardMove {
        project: String,
        card: String,
        column: String,
    },
    Waiting,
    /// A question an agent is waiting on. Only a device allowed to answer.
    Answer {
        id: String,
        allow: bool,
    },
    Chats {
        project: String,
    },
    Chat {
        project: String,
        conversation: String,
    },
    Ping,
}

/// What the host says.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "t", rename_all = "camelCase")]
pub enum RemoteOut {
    Welcome {
        device: String,
        host: String,
        typing: bool,
        answering: bool,
    },
    /// Said once, then the connection ends.
    Refused {
        why: String,
    },
    Projects {
        projects: Vec<RemoteProject>,
    },
    PaneBytes {
        pane: String,
        b64: String,
    },
    PaneClosed {
        pane: String,
    },
    Board {
        project: String,
        board: Box<Board>,
    },
    BoardChanged {
        project: String,
    },
    Waiting {
        questions: Vec<RemoteQuestion>,
    },
    Chats {
        project: String,
        conversations: Box<Conversations>,
    },
    Chat {
        project: String,
        conversation: Box<Conversation>,
    },
    /// One request could not be done; the connection stays.
    Failed {
        why: String,
    },
    Pong,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteProject {
    pub id: String,
    pub name: String,
    pub group: Option<String>,
    pub terminals: Vec<RemoteTerminal>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteTerminal {
    pub pane: String,
    /// What runs in front of it: `claude`, `zsh`.
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteQuestion {
    pub id: String,
    /// `terminal` or `chat`.
    pub from: String,
    pub project: Option<String>,
    pub tool: String,
    pub input: String,
}

/// Every message shape, for the generated contract the viewer is typed by.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RemoteShapes {
    pub incoming: Vec<RemoteIn>,
    pub outgoing: Vec<RemoteOut>,
}

/// Where Tailscale stands on this machine.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TailscaleState {
    pub installed: bool,
    /// Logged in and connected.
    pub running: bool,
    /// The machine's MagicDNS name, without the trailing dot.
    pub name: Option<String>,
    /// HTTPS certificates are on in the tailnet, so `tailscale serve` can
    /// publish the viewer with one.
    pub https: bool,
    /// Who the machine belongs to in the tailnet: the only one let in.
    pub login: Option<String>,
    /// The machine's tailnet IPv4.
    pub ip: Option<String>,
}

/// A device paired with this machine.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteDevice {
    pub id: String,
    pub name: String,
    pub typing: bool,
    pub answering: bool,
    /// Seconds since the epoch.
    pub paired_at: f64,
    pub last_seen: Option<f64>,
    pub connected: bool,
}

/// Remote, as the settings show it.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteView {
    pub enabled: bool,
    pub tailscale: TailscaleState,
    /// Where a paired device opens the viewer, once it is reachable.
    pub address: Option<String>,
    pub devices: Vec<RemoteDevice>,
    /// Why it is not reachable, when it is on and is not.
    pub problem: Option<String>,
}

/// A pairing, offered on the machine's screen and good once, briefly.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RemotePairing {
    pub code: String,
    pub url: String,
    pub qr_svg: String,
    /// Seconds since the epoch.
    pub expires_at: f64,
}
