//! The tools an orchestrator has only while the person keeps its Assistant
//! mode on. Apart from `mcp::TOOLS`, so the list without the mode is the one
//! it always was; the app says when the mode changes, and the server tells
//! the client its list changed.

use serde_json::json;

use crate::listing::Seat;
use crate::mcp::Tool;

pub(crate) const TOOLS: [Tool; 2] = [
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
