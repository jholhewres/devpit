import { useState } from 'react'

import type { RemoteDevice, RemotePairing, RemoteView } from '../gen/bindings'
import { ask, commands } from './live'
import { PrefSwitch } from './PrefSwitch'

/*
 * devpit's MCP, read by an MCP client on another computer through
 * devpit.app's relay and encrypted end to end: the board, the sessions and
 * what waits on you, never a change. Off unless turned on here; turning it
 * off, or forgetting a client, cuts it off at once.
 */

const LIMIT =
  'claude.ai and the Claude app cannot connect to it directly: they would have to hold the key that ends the encryption, so they stay out until a way keeps it end to end. Use an MCP client on a computer, such as Claude Code, through devpit there.'

export function RemoteMcp({ view, onView }: { view: RemoteView; onView: (view: RemoteView) => void }): React.JSX.Element {
  const [busy, setBusy] = useState(false)
  const [pairing, setPairing] = useState<RemotePairing | null>(null)
  const [said, setSaid] = useState<string | null>(null)
  const clients = view.devices.filter((device) => device.mcp)

  const flip = (): void => {
    setBusy(true)
    setPairing(null)
    void ask(() => commands.remoteMcpSet(!view.mcp)).then((answer) => {
      setBusy(false)
      setSaid(answer.error)
      if (answer.data) onView(answer.data)
    })
  }

  const pair = (): void =>
    void ask(() => commands.remotePair(true)).then((answer) => {
      setSaid(answer.error)
      setPairing(answer.data)
    })

  return (
    <PrefSwitch
      on={view.mcp}
      disabled={busy}
      onFlip={flip}
      title="Allow remote MCP (read-only)"
      said="An MCP client on another computer reads devpit here — the board, the sessions, what waits on you — through devpit.app, encrypted end to end. It changes nothing."
      moreLabel="How to set it up"
      more={<McpSteps />}
    >
      <span className="pref__d">{LIMIT}</span>
      {said && <span className="pref__d voice__bad">{said}</span>}
      {view.mcp && view.relay.online && !pairing && (
        <span className="voice__row">
          <button className="btn" onClick={pair}>
            Pair an MCP client
          </button>
        </span>
      )}
      {pairing && <McpPairing pairing={pairing} onDone={() => (commands.remotePairCancel(), setPairing(null))} />}
      {clients.map((client) => (
        <ClientRow key={client.id} client={client} onView={onView} />
      ))}
    </PrefSwitch>
  )
}

function McpSteps(): React.JSX.Element {
  return (
    <>
      <ol className="prefhow">
        <li>On the other computer, install devpit and sign in to the same devpit account — or let the pairing command sign it in.</li>
        <li>
          <b>Pair an MCP client</b> here and run the command it shows there. The link in it is good once, for two minutes; the key it makes stays in that computer&rsquo;s devpit folder, readable by its user alone.
        </li>
        <li>
          Add <code>devpit mcp --remote</code> to the MCP client there, for example <code>claude mcp add devpit-remote -- devpit mcp --remote</code>.
        </li>
        <li>
          <b>Forget</b> a client here, or turn this off, and it is cut off at once.
        </li>
      </ol>
      <p>
        It answers reads alone: the projects, their boards and cards, the sessions running and what waits on you. Allowing a tool, answering a session or moving a card stays on this computer or in the Remote. The protocol is in <code>docs/remote-relay.md</code>.
      </p>
    </>
  )
}

/* The command to run on the other computer, with this computer's pairing
   link in it. There is no camera to point there, so no code to scan. */
function McpPairing({ pairing, onDone }: { pairing: RemotePairing; onDone: () => void }): React.JSX.Element {
  const [copied, setCopied] = useState(false)
  const command = `devpit mcp --remote pair '${pairing.url}'`
  const copy = (): void =>
    void navigator.clipboard?.writeText(command).then(() => {
      setCopied(true)
      window.setTimeout(() => setCopied(false), 1400)
    })
  return (
    <span className="remote__code">
      <span className="pref__d">Run this on the other computer:</span>
      <code className="remote__addr">{command}</code>
      <span className="voice__row">
        <button className="btn" onClick={copy}>
          {copied ? 'Copied' : 'Copy the command'}
        </button>
        <button className="btn" onClick={onDone}>
          Done
        </button>
      </span>
      <span className="pref__d">Good once, for two minutes.</span>
    </span>
  )
}

function ClientRow({ client, onView }: { client: RemoteDevice; onView: (view: RemoteView) => void }): React.JSX.Element {
  return (
    <span className="remote__dev">
      <span className="remote__name" title="An MCP client, paired through devpit.app and known by its key">
        <i className="remote__dot" data-on={client.connected ? 'true' : undefined} />
        {client.name}
      </span>
      <span className="remote__chip" data-on="true" title="An MCP client only reads">
        read-only MCP
      </span>
      <button className="btn" onClick={() => void ask(() => commands.remoteForget(client.id)).then((answer) => answer.data && onView(answer.data))}>
        Forget
      </button>
    </span>
  )
}
