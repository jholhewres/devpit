//! The Remote through devpit.app's relay, the half that knows nothing of the
//! window: Noise end to end, the session over a WebSocket, and the machine's
//! connections to the relay. The relay joins a machine and a device and
//! carries what they encrypt for each other; it holds no key. A device can
//! be an MCP client on another machine, carried by `mcp`.
//!
//! The protocol is written down in `docs/remote-relay.md`.

pub mod link;
pub mod mcp;
pub mod noise;
pub mod relay;

pub use link::{Heard, Link, LinkError};
pub use noise::{Keys, Session};
pub use relay::{Control, Relay, RelayError, Socket};
