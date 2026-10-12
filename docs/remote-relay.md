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
from a device, 8 MiB from the machine. A photo or a recording from the phone
goes in pieces under it (`uploadPart`), each 96 KiB before base64.

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

## MCP from another machine

An MCP client on another computer — Claude Code on a laptop, say — reads
this machine's devpit through the same relay, end to end: the board, the
sessions and what waits on the person. The MCP server is this machine's;
the other computer only carries its client's messages.

- **Off unless allowed.** Settings → Remote → *Allow remote MCP (read-only)*,
  shown when devices come in through the relay. Turning it off ends every MCP
  client's session at once, and a client connecting after is refused.
- **An MCP client is a kind of device.** It pairs like a phone, from an offer
  the machine made for an MCP client — *Pair an MCP client* — so which kind a
  key is, the machine decided, never the device. The link it shows carries
  `&for=mcp`; the command to run with it on the other computer is
  `devpit mcp --remote pair '<link>'`. The tailnet's code never takes an
  offer for an MCP client.
- **The other computer.** `pair` signs that computer's devpit in to the same
  account when it is not yet (the code to approve in a browser, as Settings
  → Account does), makes an X25519 pair, and pairs over the session the
  link's machine key proves. The pair is kept in that computer's devpit home,
  `remote-mcp.json`, mode 600. A link whose origin is not the one that
  devpit signs in at is refused before anything is sent: the account's
  session would go there. `devpit mcp --remote` is then a stdio MCP server
  any MCP client there can start; `devpit mcp --remote forget` drops the key.
- **On the wire.** After the handshake, each JSON-RPC message is one sealed
  message, under the same ceilings: 256 KiB from the client, 8 MiB from the
  machine. A device's first message is the pairing, `{"t":"pair",…}`, as a
  phone's; a machine that will not serve the client says
  `{"t":"refused","why":…}` and hangs up, and the client answers every
  request still waiting with that reason. The client connects when it has
  something to send, and the machine lets a client go after 90 s with
  nothing said; it connects again on its next request.
- **Read-only.** What is offered from outside is the allowlist in
  `crates/agentapi/src/remote_mcp.rs` — `devpit_projects`, `devpit_board`,
  `devpit_card`, `devpit_sessions` and `devpit_waiting` — and every other
  tool is refused there before the app is asked. The app's side answers
  those reads and nothing else (`apps/desktop/src/remote_mcp.rs`).
  Allowing a tool, answering a session, sending a draft, moving a card stay
  on the machine or in the Remote.
- **Cut off at once.** Forgetting the client in Settings, or turning remote
  MCP off, drops its session on the machine; the relay closes the other
  side (4408) and the key opens nothing after.

**What cannot connect, for now.** claude.ai and the Claude app reach an MCP
server themselves, over HTTPS, from Anthropic's side: to read this one they
would have to hold the key that ends the encryption, which is what devpit.app
is kept from holding. They stay out until a way keeps it end to end. Settings
says so beside the switch.

## Sync's key between two machines

The relay also carries the sync key from a machine of the account to
another, in the same session. *Pair another computer* goes online under an
id and a Noise key made for that one offer; the other machine, signed in to
the same account, pastes the offer, holds the session to the key it names,
and sends its one-time secret; the first answers with the sync key and goes
offline. Nothing of the Remote's devices is involved, and Remote need not be
on. `docs/sync.md` has the rest.

## The relay's ceilings

A frame is at most 64 KiB, either way. At most 8 machines per account, 8
splices per machine, 16 per account and 2048 on the server. A device waits at
most 10 s for its machine; a splice with nothing crossing for 90 s, and a
machine's connection unanswered for 75 s, are closed. The relay logs only how
many frames and bytes a splice carried and for how long.

## Where it is

| | |
|---|---|
| `crates/remote` | the Noise session (`snow`), frames, the machine's relay client (`tungstenite` over rustls, trusting what the system trusts), and MCP carried between machines (`mcp.rs`) |
| `apps/desktop/src/remote_relay.rs` | turning it on, the machine's key, who a device is, pairing |
| `crates/agentapi/src/remote_mcp.rs` | the MCP offered from outside: the read-only allowlist |
| `apps/desktop/src/remote_mcp.rs`, `remote_mcp_client.rs` | the machine serving an MCP client; `devpit mcp --remote` on the other computer |
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

The MCP client has its own, in Rust, with the same checkout:

```sh
DEVPIT_APP_DIR=../devpit-app cargo test -p devpit-remote --test remote_mcp_through_the_relay
```

It pairs a client, reads what waits on the person, has a write refused,
checks the capture the same way, and cuts the client off by forgetting it and
by turning remote MCP off. `an_mcp_client_reads_through_a_splice.rs` runs the
same without devpit-app, through a splice of its own, in `make test`.

## The viewer devpit.app serves

devpit.app serves this repository's viewer, built for `/remote/`:

```sh
pnpm --filter ./web exec vite build --base /remote/ --outDir dist-relay
```

`remote.html`, `favicon.png` and `assets/` from `web/dist-relay` go to the
directory devpit-app's `DEVPIT_APP_REMOTE` names (its `docs/deploy.md`). A
release that changes the protocol ships its viewer there too.
