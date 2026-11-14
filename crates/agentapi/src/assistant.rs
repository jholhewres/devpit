//! The tools an orchestrator has only while the person keeps its Assistant
//! mode on. Apart from `mcp::TOOLS`, so the list without the mode is the one
//! it always was; the app says when the mode changes, and the server tells
//! the client its list changed.

use serde_json::json;

use crate::listing::Seat;
use crate::mcp::Tool;

pub(crate) const TOOLS: [Tool; 5] = [
    Tool {
        name: "devpit_schedules",
        method: "schedules",
        description: "Assistant mode only: the Schedules the person keeps — every reminder of this orchestrator and its linked projects, and every routine they set — each with when it goes off (seconds since the epoch, and a routine's zone), its project or card, whether it repeats and its state; and the schedules you proposed that wait for their click.",
        input: || json!({ "type": "object", "properties": {} }),
    },
    Tool {
        name: "devpit_schedule_propose",
        method: "schedule_propose",
        description: "Assistant mode only: propose a schedule — a routine (morning summary, CI failures, PRs waiting, cards waiting in review, at a time on some days) or a one-off reminder. Nothing is scheduled: it waits in the person's Schedules, and only their click adds it. Only the person schedules; say so when you propose one.",
        input: || json!({ "type": "object", "properties": {
            "why": { "type": "string", "description": "Why, in a line the person reads beside it." },
            "routine": { "type": "string", "enum": ["morning", "ci", "prs", "review"], "description": "A routine's kind. Leave out for a reminder." },
            "time": { "type": "string", "description": "Routine: HH:MM, in the person's zone." },
            "days": { "type": "array", "items": { "type": "integer", "minimum": 0, "maximum": 6 }, "description": "Routine: days it runs, 0 = Sunday. Leave out for every day." },
            "at": { "type": "string", "description": "Reminder: ISO 8601 with its offset, e.g. 2026-10-02T15:00:00-03:00." },
            "title": { "type": "string", "description": "Reminder: what to be reminded of." },
            "project": { "type": "string", "description": "Reminder: the project it goes on, by id or name. Leave out for your own." }
        }, "required": ["why"] }),
    },
    Tool {
        name: "devpit_heartbeat_status",
        method: "heartbeat_status",
        description: "Assistant mode only: your heartbeat — how often devpit looks for what needs the person (CI failing, a session stopped on a question for too long, a card in progress blocked), which of those it looks for, when it last looked and what it found. It speaks to the person by itself, once per thing; you need not repeat it.",
        input: || json!({ "type": "object", "properties": {} }),
    },
    Tool {
        name: "devpit_briefing",
        method: "briefing",
        description: "Assistant mode only: what happened lately, in a few lines — the sessions that ran, waited and ended, the pull requests merged, and the decisions waiting on the person. Use it when they come back or ask what they missed.",
        input: || json!({ "type": "object", "properties": {
            "hours": { "type": "integer", "minimum": 1, "maximum": 336, "description": "How far back. Defaults to 24." }
        } }),
    },
    Tool {
        name: "devpit_telegram_send",
        method: "telegram_send",
        description: "Assistant mode only: send the person a message on Telegram, when they asked to be told there or are away. As much of it goes as the chat's privacy lets out. Their answer comes back to this chat as their turn; a question for one of your sessions they answer with its own buttons, not through you.",
        input: || json!({ "type": "object", "properties": {
            "text": { "type": "string", "description": "The message, at most 4000 characters." }
        }, "required": ["text"] }),
    },
];

/// The assistant's tools `seat` is offered: all of them while its mode is
/// on, and none otherwise — an app too old to say included.
pub(crate) fn offered(seat: &Seat) -> &'static [Tool] {
    if seat.assistant == Some(true) {
        &TOOLS
    } else {
        &[]
    }
}
