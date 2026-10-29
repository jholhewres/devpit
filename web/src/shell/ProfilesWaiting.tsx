import { useCallback, useEffect, useState } from 'react'

import type { ProfileWaiting } from '../gen/bindings'
import { ask, commands } from './live'
import { profilesChanged } from './profiles'

/*
 * Profiles another computer made or changed, and what each waits for here.
 *
 * A secret never travels, so a profile that arrived needs its token typed on
 * this machine before it can start. A base URL changed elsewhere decides
 * where that token goes, so it is the person's to take — or to keep this
 * machine's own, which then travels back.
 */
export function ProfilesWaiting(): React.JSX.Element | null {
  const [waiting, setWaiting] = useState<readonly ProfileWaiting[]>([])
  const [error, setError] = useState<string | null>(null)

  const refresh = useCallback(() => {
    void ask(() => commands.agentProfilesWaiting()).then((answer) => setWaiting(answer.data ?? []))
  }, [])
  useEffect(refresh, [refresh])

  const settle = (id: string, take: boolean): void => {
    void ask(() => commands.agentProfileSettle(id, take)).then((answer) => {
      setError(answer.error)
      if (answer.data) setWaiting(answer.data)
      if (!answer.error) profilesChanged()
    })
  }

  if (waiting.length === 0 && !error) return null
  return (
    <div className="acc__note" role="status">
      {waiting.map((one) => (
        <div key={one.id}>
          {one.needs.length > 0 && (
            <p>
              <strong>{one.label}</strong> came from another computer and needs {one.needs.join(', ')} here before it can start.
              Edit it below to fill it in.
            </p>
          )}
          {one.changed.map((change) => (
            <p key={change.name}>
              <strong>{one.label}</strong>&apos;s {change.name} was changed on another computer to <code>{change.value}</code>.{' '}
              <button className="btn" onClick={() => settle(one.id, true)}>
                Use it
              </button>{' '}
              <button className="btn" onClick={() => settle(one.id, false)}>
                Keep this one
              </button>
            </p>
          ))}
        </div>
      ))}
      {error && <p role="alert">{error}</p>}
    </div>
  )
}
