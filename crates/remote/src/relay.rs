//! The machine's side of devpit.app's relay: one connection that stays open
//! and hears which devices are waiting, and one more per device, for the
//! relay to join to it. Every connection goes out from here; nothing listens.

use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use tungstenite::client::IntoClientRequest;
use tungstenite::handshake::HandshakeError;
use tungstenite::protocol::WebSocketConfig;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Connector, Message, WebSocket};

use crate::noise::MOST_FRAME;

pub type Socket = WebSocket<MaybeTlsStream<TcpStream>>;

/// How long connecting, and the WebSocket's own handshake, may take.
const CONNECT_WITHIN: Duration = Duration::from_secs(10);
/// A write that cannot go out in this long means the connection is gone.
const WRITE_WITHIN: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelayError {
    /// The account's session was refused: signed out, or ended elsewhere.
    SignedOut,
    /// Past a ceiling of the relay's.
    TooMany,
    /// The relay answered, and not with a WebSocket.
    Refused(u16),
    /// Nothing answered, or the connection broke.
    Unreachable(String),
    /// The machine or splice id is not one devpit makes.
    BadId,
}

impl std::fmt::Display for RelayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RelayError::SignedOut => {
                f.write_str("devpit.app refused this machine's session — sign in again")
            }
            RelayError::TooMany => {
                f.write_str("devpit.app has too many connections of this account")
            }
            RelayError::Refused(status) => write!(f, "devpit.app answered {status}"),
            RelayError::Unreachable(why) => write!(f, "devpit.app could not be reached: {why}"),
            RelayError::BadId => f.write_str("not an id devpit makes"),
        }
    }
}

/// An id that can go in an address as it is.
pub fn id_ok(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
}

/// The relay's address for `path`: `https://` becomes `wss://`.
pub fn socket_url(origin: &str, path: &str) -> Option<String> {
    let origin = origin.trim_end_matches('/');
    let rest = origin
        .strip_prefix("https://")
        .map(|rest| ("wss://", rest))
        .or_else(|| origin.strip_prefix("http://").map(|rest| ("ws://", rest)))?;
    Some(format!("{}{}{path}", rest.0, rest.1))
}

/// rustls on aws-lc, trusting what the system trusts — what reqwest uses for
/// devpit.app already.
fn tls() -> Result<Arc<rustls::ClientConfig>, RelayError> {
    static CONFIG: OnceLock<Result<Arc<rustls::ClientConfig>, String>> = OnceLock::new();
    CONFIG
        .get_or_init(|| {
            use rustls_platform_verifier::BuilderVerifierExt as _;
            let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
            rustls::ClientConfig::builder_with_provider(provider)
                .with_safe_default_protocol_versions()
                .map_err(|err| err.to_string())?
                .with_platform_verifier()
                .map(|builder| Arc::new(builder.with_no_client_auth()))
                .map_err(|err| err.to_string())
        })
        .clone()
        .map_err(RelayError::Unreachable)
}

/// This machine, as the relay knows it.
#[derive(Clone)]
pub struct Relay {
    origin: String,
    token: String,
    host: String,
}

impl std::fmt::Debug for Relay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The account's token never reaches a log.
        write!(
            f,
            "Relay {{ origin: {}, host: {} }}",
            self.origin, self.host
        )
    }
}

impl Relay {
    pub fn new(origin: &str, token: &str, host: &str) -> Relay {
        Relay {
            origin: origin.to_owned(),
            token: token.to_owned(),
            host: host.to_owned(),
        }
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    fn open(&self, path: &str) -> Result<Socket, RelayError> {
        let url = socket_url(&self.origin, path)
            .ok_or_else(|| RelayError::Unreachable("not an http(s) address".to_owned()))?;
        let mut request = url
            .as_str()
            .into_client_request()
            .map_err(|err| RelayError::Unreachable(err.to_string()))?;
        let bearer = format!("Bearer {}", self.token)
            .parse()
            .map_err(|_| RelayError::SignedOut)?;
        request.headers_mut().insert("authorization", bearer);
        request.headers_mut().insert(
            "user-agent",
            concat!("devpit/", env!("CARGO_PKG_VERSION"))
                .parse()
                .map_err(|_| RelayError::BadId)?,
        );
        let uri = request.uri().clone();
        let secure = uri.scheme_str() == Some("wss");
        let host = uri
            .host()
            .ok_or_else(|| RelayError::Unreachable("no host".to_owned()))?;
        let port = uri.port_u16().unwrap_or(if secure { 443 } else { 80 });
        let address = (host.trim_matches(['[', ']']), port)
            .to_socket_addrs()
            .map_err(|err| RelayError::Unreachable(err.to_string()))?
            .next()
            .ok_or_else(|| RelayError::Unreachable("no address".to_owned()))?;
        let stream = TcpStream::connect_timeout(&address, CONNECT_WITHIN)
            .map_err(|err| RelayError::Unreachable(err.to_string()))?;
        let _ = stream.set_nodelay(true);
        let _ = stream.set_read_timeout(Some(CONNECT_WITHIN));
        let _ = stream.set_write_timeout(Some(WRITE_WITHIN));
        let connector = if secure {
            Connector::Rustls(tls()?)
        } else {
            Connector::Plain
        };
        // The relay's own ceiling, kept on this side too.
        let config = WebSocketConfig::default()
            .max_message_size(Some(MOST_FRAME + 1))
            .max_frame_size(Some(MOST_FRAME + 1));
        match tungstenite::client_tls_with_config(request, stream, Some(config), Some(connector)) {
            Ok((socket, _)) => Ok(socket),
            Err(HandshakeError::Failure(tungstenite::Error::Http(answer))) => {
                Err(match answer.status().as_u16() {
                    401 => RelayError::SignedOut,
                    429 => RelayError::TooMany,
                    status => RelayError::Refused(status),
                })
            }
            Err(err) => Err(RelayError::Unreachable(err.to_string())),
        }
    }

    /// The connection that keeps this machine online.
    pub fn online(&self) -> Result<Control, RelayError> {
        if !id_ok(&self.host) {
            return Err(RelayError::BadId);
        }
        let socket = self.open(&format!("/v1/relay/host?host={}", self.host))?;
        timeout(&socket, Duration::from_millis(500));
        Ok(Control { socket })
    }

    /// This machine's side of splice `id`, for the relay to join to the
    /// device waiting under it.
    pub fn splice(&self, id: &str) -> Result<Socket, RelayError> {
        if !id_ok(&self.host) || !id_ok(id) {
            return Err(RelayError::BadId);
        }
        self.open(&format!("/v1/relay/splice?host={}&id={id}", self.host))
    }
}

/// How long one read of `socket` waits before it says nothing came.
pub fn timeout(socket: &Socket, wait: Duration) {
    let stream = match socket.get_ref() {
        MaybeTlsStream::Plain(stream) => stream,
        MaybeTlsStream::Rustls(stream) => stream.get_ref(),
        _ => return,
    };
    let _ = stream.set_read_timeout(Some(wait));
}

/// The machine's connection to the relay.
pub struct Control {
    socket: Socket,
}

impl Control {
    /// The splice id of the next device waiting, or nothing yet.
    pub fn waiting(&mut self) -> Result<Option<String>, RelayError> {
        let read = self.socket.read();
        // The pongs owed to the relay's pings go out here.
        let _ = self.socket.flush();
        match read {
            Ok(Message::Text(text)) => {
                let said: serde_json::Value = serde_json::from_str(&text).unwrap_or_default();
                Ok((said["t"] == "viewer")
                    .then(|| said["splice"].as_str().map(str::to_owned))
                    .flatten()
                    .filter(|id| id_ok(id)))
            }
            Ok(Message::Close(frame)) => Err(match frame.map(|frame| u16::from(frame.code)) {
                Some(4401) => RelayError::SignedOut,
                _ => RelayError::Unreachable("closed by devpit.app".to_owned()),
            }),
            Ok(_) => Ok(None),
            Err(tungstenite::Error::Io(err))
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                Ok(None)
            }
            Err(err) => Err(RelayError::Unreachable(err.to_string())),
        }
    }

    pub fn close(mut self) {
        let _ = self.socket.close(None);
        let _ = self.socket.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::{id_ok, socket_url};

    #[test]
    fn the_relay_is_reached_over_the_origins_own_scheme() {
        assert_eq!(
            socket_url("https://devpit.app/", "/v1/relay/host?host=h").as_deref(),
            Some("wss://devpit.app/v1/relay/host?host=h")
        );
        assert_eq!(
            socket_url("http://127.0.0.1:8787", "/v1/relay/view").as_deref(),
            Some("ws://127.0.0.1:8787/v1/relay/view")
        );
        assert_eq!(socket_url("ftp://devpit.app", "/x"), None);
    }

    #[test]
    fn an_id_goes_into_an_address_only_as_devpit_makes_it() {
        assert!(id_ok("host_01J9ZK3Q"));
        assert!(id_ok("0123456789abcdef0123456789abcdef"));
        for bad in ["", "a&id=b", "a b", "../x", &"x".repeat(65)] {
            assert!(!id_ok(bad), "{bad}");
        }
    }
}
