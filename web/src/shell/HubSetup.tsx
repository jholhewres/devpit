import { useEffect, useState } from 'react'

import type { HubStatus } from '../gen/bindings'
import { ask, commands } from './live'

/*
 * devpit.app as a channel: it e-mails what waits on you and wakes your phone,
 * set up at devpit.app/account. What leaves this machine is only that
 * something waits, unless titles are on; a button pressed there is done here.
 */
export function HubSetup({ onChanged }: { onChanged: () => void }): React.JSX.Element {
  const [status, setStatus] = useState<HubStatus | null>(null)
  useEffect(() => {
    void ask(() => commands.hubStatus()).then((answer) => setStatus(answer.data))
  }, [])
  const set = (on: boolean, titles: boolean): void =>
    void ask(() => commands.hubSet(on, titles)).then((answer) => {
      if (answer.data) setStatus(answer.data)
      onChanged()
    })
  if (!status) return <></>
  return (
    <div className="pref">
      <span className="pref__body">
        <span className="pref__t">devpit.app — e-mail and push</span>
        <span className="pref__d">
          devpit.app e-mails what waits on you and wakes your phone; choose which at{' '}
          <button className="plusl" onClick={() => void ask(() => commands.urlOpen('https://devpit.app/account'))}>
            devpit.app/account
          </button>
          . A button pressed there is done here, when this computer is on.
          {!status.signedIn && ' Sign in to devpit.app on this computer first (Settings → Account).'}
        </span>
        <label className="voice__row">
          <input type="checkbox" disabled={!status.signedIn} checked={status.on} onChange={(event) => set(event.target.checked, status.titles)} />
          Send notices through devpit.app
        </label>
        {status.on && (
          <label className="voice__row">
            <input type="checkbox" checked={status.titles} onChange={(event) => set(status.on, event.target.checked)} />
            Say which session, card or project — e-mail is not end-to-end encrypted
          </label>
        )}
      </span>
    </div>
  )
}
