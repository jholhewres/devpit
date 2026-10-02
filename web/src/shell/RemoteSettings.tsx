import { useEffect, useState } from 'react'

import type { RemotePairing, RemoteView } from '../gen/bindings'
import { ask, commands } from './live'
import { PrefSwitch } from './PrefSwitch'
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
    <>
      <PrefSwitch on={view.enabled} onFlip={busy ? () => undefined : flip} title="Remote" said={<>Reach this machine from your phone or another computer, over your Tailscale tailnet. {RISK}</>} />
      {view.enabled && (
        <div className="pref pref--stack">
          <span className="pref__body">
            <span className="pref__d">
              {view.address ? <>Open <code>{view.address}</code> on a device in your tailnet, signed in as {tailscale.login}.</> : (view.problem ?? 'Not reachable yet.')}
            </span>
            {missing && <span className="pref__d">{missing}</span>}
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
              <span className="remote__dev" key={device.id}>
                <span className="remote__name">
                  <i className="remote__dot" data-on={device.connected ? 'true' : undefined} />
                  {device.name}
                </span>
                <label className="remote__cap">
                  <input type="checkbox" checked readOnly disabled /> watch
                </label>
                <label className="remote__cap">
                  <input type="checkbox" checked={device.typing} onChange={(event) => void ask(() => commands.remoteAllow(device.id, event.target.checked, device.answering)).then((answer) => answer.data && setView(answer.data))} /> type
                </label>
                <label className="remote__cap">
                  <input type="checkbox" checked={device.answering} onChange={(event) => void ask(() => commands.remoteAllow(device.id, device.typing, event.target.checked)).then((answer) => answer.data && setView(answer.data))} /> answer
                </label>
                <button className="btn" onClick={() => void ask(() => commands.remoteForget(device.id)).then((answer) => answer.data && setView(answer.data))}>
                  Forget
                </button>
              </span>
            ))}
          </span>
        </div>
      )}
    </>
  )
}
