//! Noise, as the Remote speaks it through the relay.
//!
//! `Noise_XX_25519_ChaChaPoly_BLAKE2s`: each end proves its static key, and
//! the session's keys come from ephemeral ones, so a key taken later opens
//! nothing said before. The prologue names the protocol and the machine, so a
//! handshake meant for one machine does not complete with another.
//!
//! One Noise message is at most 65535 bytes, and a board or a chat can be
//! more, so a message travels as frames: each frame's plaintext is a flag —
//! more to come, or the last — and a chunk. The nonce of each frame is its
//! place in the session, so a frame dropped, repeated or moved by anyone on
//! the way does not open, and the session ends there.

use std::fmt;

/// The handshake both ends speak.
pub const PATTERN: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";
/// The most one Noise message — one frame — may be.
pub const MOST_FRAME: usize = 65535;
/// ChaCha20-Poly1305's tag.
const TAG: usize = 16;
/// What one frame carries of a message, after its flag.
pub const MOST_CHUNK: usize = MOST_FRAME - TAG - 1;
/// Frames a session carries before it must be made again. Far past any real
/// session, and inside the 2^53 the viewer's counter holds exactly.
pub const MOST_FRAMES: u64 = 1 << 48;

const MORE: u8 = 0;
const LAST: u8 = 1;

/// What binds a handshake to devpit's Remote and to one machine.
pub fn prologue(host: &str) -> Vec<u8> {
    format!("devpit-remote/1 {host}").into_bytes()
}

#[derive(Debug)]
pub enum Error {
    /// The handshake or a frame did not open: wrong keys, a frame changed,
    /// dropped or repeated on the way.
    Broken,
    /// A message, or a frame, past what this end takes.
    TooBig,
    /// The session carried all it may; a new one is a new handshake.
    Worn,
    /// Spoken out of turn: a frame before the handshake ended, or after.
    OutOfTurn,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Error::Broken => "a frame did not open",
            Error::TooBig => "a message past the ceiling",
            Error::Worn => "the session carried all it may",
            Error::OutOfTurn => "a frame out of turn",
        })
    }
}

impl std::error::Error for Error {}

fn params() -> snow::params::NoiseParams {
    PATTERN.parse().expect("a pattern snow knows")
}

/// A static X25519 key pair.
#[derive(Clone, PartialEq, Eq)]
pub struct Keys {
    pub secret: [u8; 32],
    pub public: [u8; 32],
}

impl fmt::Debug for Keys {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The secret never reaches a log, a panic or a test's output.
        write!(f, "Keys {{ public: {} }}", hex(&self.public))
    }
}

impl Keys {
    pub fn generate() -> Result<Keys, Error> {
        let pair = snow::Builder::new(params())
            .generate_keypair()
            .map_err(|_| Error::Broken)?;
        Ok(Keys {
            secret: pair.private.try_into().map_err(|_| Error::Broken)?,
            public: pair.public.try_into().map_err(|_| Error::Broken)?,
        })
    }
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// 32 bytes, from 64 hex digits.
pub fn unhex32(text: &str) -> Option<[u8; 32]> {
    if text.len() != 64 || !text.is_ascii() {
        return None;
    }
    let mut out = [0u8; 32];
    for (at, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[at * 2..at * 2 + 2], 16).ok()?;
    }
    Some(out)
}

/// A handshake under way.
pub struct Handshake {
    state: snow::HandshakeState,
}

impl Handshake {
    /// The machine's side: it answers.
    pub fn responder(keys: &Keys, host: &str) -> Result<Handshake, Error> {
        Self::build(keys, host, false, None)
    }

    /// The device's side: it begins.
    pub fn initiator(keys: &Keys, host: &str) -> Result<Handshake, Error> {
        Self::build(keys, host, true, None)
    }

    /// For a test vector alone: the ephemeral key fixed, so the bytes are known.
    pub fn with_ephemeral(
        keys: &Keys,
        host: &str,
        initiator: bool,
        ephemeral: &[u8; 32],
    ) -> Result<Handshake, Error> {
        Self::build(keys, host, initiator, Some(ephemeral))
    }

    fn build(
        keys: &Keys,
        host: &str,
        initiator: bool,
        ephemeral: Option<&[u8; 32]>,
    ) -> Result<Handshake, Error> {
        let prologue = prologue(host);
        let mut builder = snow::Builder::new(params())
            .local_private_key(&keys.secret)
            .and_then(|builder| builder.prologue(&prologue))
            .map_err(|_| Error::Broken)?;
        if let Some(ephemeral) = ephemeral {
            builder = builder.fixed_ephemeral_key_for_testing_only(ephemeral);
        }
        let state = if initiator {
            builder.build_initiator()
        } else {
            builder.build_responder()
        }
        .map_err(|_| Error::Broken)?;
        Ok(Handshake { state })
    }

    /// Whether the next message is this end's to write.
    pub fn writes_next(&self) -> bool {
        self.state.is_my_turn()
    }

    pub fn finished(&self) -> bool {
        self.state.is_handshake_finished()
    }

    pub fn write(&mut self) -> Result<Vec<u8>, Error> {
        if !self.writes_next() || self.finished() {
            return Err(Error::OutOfTurn);
        }
        let mut out = vec![0u8; MOST_FRAME];
        let len = self
            .state
            .write_message(&[], &mut out)
            .map_err(|_| Error::Broken)?;
        out.truncate(len);
        Ok(out)
    }

    /// A handshake message in. Its payload is empty; one that is not is not
    /// devpit's.
    pub fn read(&mut self, message: &[u8]) -> Result<(), Error> {
        if self.writes_next() || self.finished() {
            return Err(Error::OutOfTurn);
        }
        if message.len() > MOST_FRAME {
            return Err(Error::TooBig);
        }
        let mut out = vec![0u8; MOST_FRAME];
        let len = self
            .state
            .read_message(message, &mut out)
            .map_err(|_| Error::Broken)?;
        if len != 0 {
            return Err(Error::Broken);
        }
        Ok(())
    }

    /// The other end's static key, once it has said it.
    pub fn peer(&self) -> Option<[u8; 32]> {
        self.state.get_remote_static()?.try_into().ok()
    }

    /// The session, taking messages up to `most` bytes from the other end.
    pub fn into_session(self, most: usize) -> Result<Session, Error> {
        let peer = self.peer().ok_or(Error::OutOfTurn)?;
        let transport = self
            .state
            .into_transport_mode()
            .map_err(|_| Error::OutOfTurn)?;
        Ok(Session {
            transport,
            peer,
            most,
            inbox: Vec::new(),
        })
    }
}

/// An open session: messages sealed into frames, frames opened into messages.
pub struct Session {
    transport: snow::TransportState,
    peer: [u8; 32],
    /// The largest message taken from the other end.
    most: usize,
    /// A message whose frames have not all come.
    inbox: Vec<u8>,
}

impl Session {
    pub fn peer(&self) -> [u8; 32] {
        self.peer
    }

    /// `message` as the frames that carry it, in order.
    pub fn seal(&mut self, message: &[u8]) -> Result<Vec<Vec<u8>>, Error> {
        let chunks: Vec<&[u8]> = if message.is_empty() {
            vec![&[][..]]
        } else {
            message.chunks(MOST_CHUNK).collect()
        };
        let last = chunks.len() - 1;
        let mut frames = Vec::with_capacity(chunks.len());
        let mut plain = Vec::with_capacity(MOST_CHUNK + 1);
        for (at, chunk) in chunks.into_iter().enumerate() {
            if self.transport.sending_nonce() >= MOST_FRAMES {
                return Err(Error::Worn);
            }
            plain.clear();
            plain.push(if at == last { LAST } else { MORE });
            plain.extend_from_slice(chunk);
            let mut out = vec![0u8; plain.len() + TAG];
            let len = self
                .transport
                .write_message(&plain, &mut out)
                .map_err(|_| Error::Broken)?;
            out.truncate(len);
            frames.push(out);
        }
        Ok(frames)
    }

    /// One frame in: the whole message once its last frame is, or nothing yet.
    pub fn open(&mut self, frame: &[u8]) -> Result<Option<Vec<u8>>, Error> {
        if frame.len() > MOST_FRAME {
            return Err(Error::TooBig);
        }
        if frame.len() < TAG + 1 {
            return Err(Error::Broken);
        }
        if self.transport.receiving_nonce() >= MOST_FRAMES {
            return Err(Error::Worn);
        }
        let mut plain = vec![0u8; frame.len() - TAG];
        let len = self
            .transport
            .read_message(frame, &mut plain)
            .map_err(|_| Error::Broken)?;
        let (flag, chunk) = plain[..len].split_first().ok_or(Error::Broken)?;
        // Checked before it is kept: the ceiling bounds what this end holds.
        if self.inbox.len() + chunk.len() > self.most {
            return Err(Error::TooBig);
        }
        self.inbox.extend_from_slice(chunk);
        match *flag {
            LAST => Ok(Some(std::mem::take(&mut self.inbox))),
            MORE => Ok(None),
            _ => Err(Error::Broken),
        }
    }
}

#[cfg(test)]
#[path = "noise_tests.rs"]
mod tests;
