import { useEffect, useState } from 'react'

import type { RemoteActivity, RemoteDevice, RemotePairing, RemoteView } from '../gen/bindings'
import { ask, commands } from './live'
import { PlusLink } from './PlusLink'
import { PrefSwitch } from './PrefSwitch'
import { launcherLink } from './remoteLauncher'
import { onCarried } from './window'

/*
 * Remote: this machine, from the person's other devices on their tailnet.
 * Off unless turned on; every device paired here with a code read off this
 * screen; what each may do besides watching granted here, and only here.
 */

const RISK = 'Anyone with a paired device can see the terminals on this machine — and, if you allow that device, type in them and answer for you.'

export function RemoteSettings(): React.JSX.Element {
  const [view, setView] = useState<RemoteView | null>(null)
  const [pairing, setPairing] = useState<RemotePairing | null>(null)
  const [busy, setBusy] = useState(false)
  const [said, setSaid] = useState<string | null>(null)

  const read = (): void => void ask(() => commands.remoteRead()).then((answer) => answer.data && setView(answer.data))
  useEffect(read, [])
  useEffect(() => onCarried<null>('remote:changed', () => (read(), setPairing(null))), [])
  useEffect(() => onCarried<number>('remote:viewers', read), [])

  const flip = (): void => {
    if (!view) return
    setBusy(true)
    void ask(() => commands.remoteSet(!view.enabled)).then((answer) => {
      setBusy(false)
      setSaid(answer.error)
      if (answer.data) setView(answer.data)
      if (!answer.data?.enabled) setPairing(null)
    })
  }

  const pair = (): void =>
    void ask(() => commands.remotePair()).then((answer) => {
      setSaid(answer.error)
      setPairing(answer.data)
    })

  if (!view) return <></>
  const tailscale = view.tailscale
  const missing = !tailscale.installed
    ? 'Install Tailscale on this machine and on your devices.'
    : !tailscale.running
      ? 'Log in to Tailscale on this machine.'
      : !tailscale.https
        ? 'Reached over your tailnet’s own encryption. Turn on HTTPS certificates in the tailnet’s admin for an https address.'
        : null

  return (
    <PrefSwitch
      on={view.enabled}
      disabled={busy}
      onFlip={flip}
      title="Remote"
      said={<>Reach this machine from your phone or another computer, over your Tailscale tailnet. {RISK}</>}
      moreLabel="How to set it up"
      more={
        <>
          <ol className="prefhow">
            <li>Install Tailscale on the phone or computer and sign in as the owner of this machine{tailscale.login && <> ({tailscale.login})</>}. Anyone else in the tailnet is refused.</li>
            <li>Open this machine&rsquo;s address on that device.</li>
            <li><b>Pair a device</b> here, then scan the code or type it there. It is good once, for two minutes.</li>
            <li>Choose what the device may do. Watching is always allowed; <b>type</b> sends keys to terminals and moves cards; <b>answer</b> allows or denies a question an agent is waiting on. Chats are read there, not written.</li>
          </ol>
          <p>
            The status strip says <i>Watched by 1 device</i> while one is connected, and a click drops it. Who connected and what they did, never what was on screen, is kept in <code>~/.devpit/remote-log.jsonl</code>.
          </p>
        </>
      }
    >
      {view.enabled && (
        <>
          {view.address ? <RemoteAddress address={view.address} login={tailscale.login} /> : <span className="pref__d">{view.problem ?? 'Not reachable yet.'}</span>}
          {missing && <span className="pref__d">{missing}</span>}
          {!tailscale.installed && <PlusLink from="app-remote" said="Without Tailscale, a relay encrypted end to end is planned for devpit Plus." />}
          {said && <span className="pref__d voice__bad">{said}</span>}
          {view.address && !pairing && (
            <span className="voice__row">
              <button className="btn" onClick={pair}>
                Pair a device
              </button>
            </span>
          )}
          {pairing && (
            <span className="remote__pair">
              <span className="remote__qr" dangerouslySetInnerHTML={{ __html: pairing.qrSvg }} />
              <span className="remote__code">
                <span className="pref__d">Scan it, or open the address and type</span>
                <code className="remote__big">{pairing.code}</code>
                <span className="pref__d">Good once, for two minutes.</span>
                <button className="btn" onClick={() => (commands.remotePairCancel(), setPairing(null))}>
                  Done
                </button>
              </span>
            </span>
          )}
          {view.devices.map((device) => (
            <DeviceRow key={device.id} device={device} onView={setView} />
          ))}
          {view.devices.length > 0 && <Activity />}
        </>
      )}
    </PrefSwitch>
  )
}

/* The address to type on the other device, one click from the clipboard:
   copied by hand off a settings pane, a tailnet name is easy to get wrong. */
function RemoteAddress({ address, login }: { address: string; login: string | null }): React.JSX.Element {
  const [copied, setCopied] = useState(false)
  const copy = (): void =>
    void navigator.clipboard?.writeText(address).then(() => {
      setCopied(true)
      window.setTimeout(() => setCopied(false), 1400)
    })
  return (
    <>
      <span className="voice__row">
        <code className="remote__addr">{address}</code>
        <button className="btn" onClick={copy}>
          {copied ? 'Copied' : 'Copy'}
        </button>
      </span>
      <span className="pref__d">Open it on a device in your tailnet{login && <>, signed in as {login}</>}.</span>
      <LauncherRow address={address} />
    </>
  )
}

/* What paired devices did lately, from the Remote's log — never what they saw. */
function Activity(): React.JSX.Element {
  const [entries, setEntries] = useState<readonly RemoteActivity[] | null>(null)
  const show = (): void => void ask(() => commands.remoteActivity()).then((answer) => setEntries(answer.data?.entries ?? []))
  if (!entries)
    return (
      <span className="voice__row">
        <button className="btn" onClick={show}>
          Show activity
        </button>
      </span>
    )
  return (
    <span className="remote__log">
      {entries.length === 0 && <span className="pref__d">Nothing done from a device yet.</span>}
      {entries.map((one, at) => (
        <span className="pref__d" key={at}>
          {new Date((one.at ?? 0) * 1000).toLocaleString(undefined, { dateStyle: 'short', timeStyle: 'short' })} · {one.device} · {one.what}
        </span>
      ))}
      <button className="btn" onClick={() => setEntries(null)}>
        Hide activity
      </button>
    </span>
  )
}

/* devpit.app/remote opens this machine's own address; it is handed only that,
   in the fragment, and needs the tailnet's HTTPS one. */
function LauncherRow({ address }: { address: string }): React.JSX.Element {
  const link = launcherLink(address)
  if (!link) return <span className="pref__d">Adding it to devpit.app/remote needs the tailnet&rsquo;s HTTPS address.</span>
  return (
    <span className="voice__row">
      <button className="btn" onClick={() => void ask(() => commands.urlOpen(link))} title="Opens devpit.app/remote with this machine's address; nothing else is sent">
        Add to devpit.app
      </button>
    </span>
  )
}

/* One paired device and what it may do. Watching is not a choice, so it is
   drawn as a fact beside the two that are. */
function DeviceRow({ device, onView }: { device: RemoteDevice; onView: (view: RemoteView) => void }): React.JSX.Element {
  const allow = (typing: boolean, answering: boolean): void =>
    void ask(() => commands.remoteAllow(device.id, typing, answering)).then((answer) => answer.data && onView(answer.data))
  return (
    <span className="remote__dev">
      <span className="remote__name">
        <i className="remote__dot" data-on={device.connected ? 'true' : undefined} />
        {device.name}
      </span>
      <span className="remote__chips" role="group" aria-label={`What ${device.name} may do`}>
        <span className="remote__chip" data-on="true" title="Watching is always allowed">
          watch
        </span>
        <button className="remote__chip" aria-pressed={device.typing} title="Send keys to terminals and move cards" onClick={() => allow(!device.typing, device.answering)}>
          type
        </button>
        <button className="remote__chip" aria-pressed={device.answering} title="Allow or deny a question an agent is waiting on" onClick={() => allow(device.typing, !device.answering)}>
          answer
        </button>
      </span>
      <button className="btn" onClick={() => void ask(() => commands.remoteForget(device.id)).then((answer) => answer.data && onView(answer.data))}>
        Forget
      </button>
    </span>
  )
}
