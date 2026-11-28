//! The tools an orchestrator has only while the person keeps its Agents mode
//! on. Apart from `mcp::TOOLS`, so the list without the mode is the one it
//! always was; the app says when the mode changes, and the server tells the
//! client its list changed.

use serde_json::json;

use crate::listing::Seat;
use crate::mcp::Tool;

pub(crate) const TOOLS: [Tool; 1] = [Tool {
    name: "devpit_agents",
    method: "agents",
    description: "Agents mode only: your agents, the ones you hand work to, each with its handle, name and role. You plan and delegate; they do the work. Only the person creates or changes an agent.",
    input: || json!({ "type": "object", "properties": {} }),
}];

/// The agents' tools `seat` is offered: all of them while its mode is on,
/// and none otherwise — an app too old to say included.
pub(crate) fn offered(seat: &Seat) -> &'static [Tool] {
    if seat.agents == Some(true) {
        &TOOLS
    } else {
        &[]
    }
}
