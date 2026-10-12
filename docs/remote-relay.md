# The Remote through devpit.app's relay

The Remote's second way in, beside the tailnet: a phone on any network
reaches this machine through devpit.app, with nothing to install. devpit.app
joins the two and passes along what they say. It holds no key and cannot read
it.

## What each party sees

| | sees | cannot |
|---|---|---|
| devpit.app's relay | which account's machine and device are connected, when, and how many bytes cross | read or change a frame; join a device to another account's machine |
| someone with the relay's logs or database | the same counts | anything more: nothing else is kept |
| a device without a pairing | that the machine is online, if it is signed in to the same account | finish a session: the machine refuses a key it does not know |
| devpit.app serving a malicious viewer | whatever that page then does with the device's key | — this is the limit of any end-to-end scheme in a web page |

The last row is the honest one. The viewer at `devpit.app/remote` is
JavaScript devpit.app hands out; a server that chose to attack its own users
could hand out one that leaks the device's key. Settings says so beside the
choice. Over Tailscale the viewer comes from the machine itself and this row
does not exist.

## Connections

Every connection goes out — from the machine and from the device. Nothing
listens on the machine for the relay.

| | path | credential |
|---|---|---|
| the machine, while Remote is on | `GET /v1/relay/host?host=<id>` | the desktop's account session, `Authorization: Bearer` |
| the machine's side of one device | `GET /v1/relay/splice?host=<id>&id=<splice>` | the same |
| a device | `GET /v1/relay/view?host=<id>` | the browser's devpit.app cookie, accepted only from devpit.app's own page |

The machine's connection hears `{"t":"viewer","splice":"<id>"}` for each
device that asks for it, and opens a splice for it. From then on the relay
pipes the two sockets into each other, a binary frame at a time.

A device is joined only to a machine of its own account. The relay answers a
page with close codes it can act on: 4401 sign in, 4404 the machine is not
connected (also the answer for another account's machine), 4408 the other end
left, 4429 too many connections, 4504 the machine did not answer in time.

## Keys

- **The machine's key** is an X25519 pair made the first time the relay is
  used, kept with the machine's relay id in `~/.devpit/remote-relay.json`,
  mode 600.
- **A device's key** is an X25519 pair the viewer makes once and keeps in the
  browser's storage for devpit.app.
- **Pairing** is the QR code Settings shows, or the link under it:
  `https://devpit.app/remote#relay=<id>&key=<machine key>&secret=<32 bytes>&name=<machine>`.
  It is a fragment, which browsers never send. The key is what the device
  holds the machine to; the secret is one-time, good for two minutes, and
  burned after five wrong tries, like the tailnet's code.

## The session

Noise, `Noise_XX_25519_ChaChaPoly_BLAKE2s`, the device as initiator, with the
prologue `devpit-remote/1 <machine id>` and empty handshake payloads:

```
device → machine   e
machine → device   e, ee, s, es
device → machine   s, se
```

- The device reads the machine's static key from the second message and goes
  on only if it is the key its QR code named — before it says who it is.
- The machine reads the device's static key from the third message. A key it
  paired is that device. A key it does not know must send, as its first
  sealed message, `{"t":"pair","secret":…,"name":…}` with the QR code's
  secret, and is then paired by that key; anything else is refused.
- XX gives both: each end proves its static key, and the session's keys come
  from ephemeral ones, so a key taken later opens nothing said before.

After the handshake, each JSON message of the Remote's protocol
(`RemoteIn`/`RemoteOut`, the same as on the tailnet) is sealed as one or more
frames. One frame is one Noise message, at most 65535 bytes, and one
WebSocket binary frame; its plaintext is a flag byte — `0` more follows, `1`
last — and a chunk. A message is held to a ceiling before it is kept: 256 KiB
from a device, 8 MiB from the machine.

Each frame's nonce is its place in the session, so a frame dropped, repeated,
reordered or changed on the way does not open, and the session ends there.
A session ends after 2^48 frames — far past any real one — and reconnecting
is a new handshake.

## Unpairing

Forgetting a device in Settings ends its connections. Through the relay that
drops the session and its keys and closes the machine's side; the relay closes
the device's side the moment it does (4408). The device's next handshake
finishes at Noise level and is refused by the machine: its key is no longer
one it knows, and its secret was used.

## The relay's ceilings

A frame is at most 64 KiB, either way. At most 8 machines per account, 8
splices per machine, 16 per account and 2048 on the server. A device waits at
most 10 s for its machine; a splice with nothing crossing for 90 s, and a
machine's connection unanswered for 75 s, are closed. The relay logs only how
many frames and bytes a splice carried and for how long.

## Where it is

| | |
|---|---|
| `crates/remote` | the Noise session (`snow`), frames, and the machine's relay client (`tungstenite` over rustls, trusting what the system trusts) |
| `apps/desktop/src/remote_relay.rs` | turning it on, the machine's key, who a device is, pairing |
| `web/src/remote/noise.ts` | the viewer's Noise, on `@noble/curves`, `@noble/ciphers` and `@noble/hashes` |
| `web/src/remote/relay.ts` | the viewer's relay transport |
| devpit-app `crates/relay` | the relay itself |

No primitive is devpit's own. The viewer's Noise bookkeeping is held byte for
byte to a transcript `snow` made with fixed keys,
`crates/remote/tests/noise-xx.json`: `the_noise_vector_holds.rs` holds snow to
it and `noise.test.ts` holds the viewer to it.

The whole path — devpit-app's relay, the machine's relay client and the
viewer's transport, on loopback — runs with a devpit-app checkout beside this
one:

```sh
DEVPIT_APP_DIR=../devpit-app pnpm --filter ./web exec vitest run src/remote/relay.loopback.test.ts
```

It pairs, reads a message longer than one frame, checks every frame the relay
carried for the words that crossed and against keys of its own making, has a
device of another account refused, and unpairs the device to see it dropped
and then refused.

## The viewer devpit.app serves

devpit.app serves this repository's viewer, built for `/remote/`:

```sh
pnpm --filter ./web exec vite build --base /remote/ --outDir dist-relay
```

`remote.html`, `favicon.png` and `assets/` from `web/dist-relay` go to the
directory devpit-app's `DEVPIT_APP_REMOTE` names (its `docs/deploy.md`). A
release that changes the protocol ships its viewer there too.
