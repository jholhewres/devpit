import { useCallback, useEffect, useState } from 'react'

import type { Profile } from '../gen/bindings'
import { ask, commands } from './live'

/*
 * The agent CLIs this computer actually has.
 *
 * The two fixed rows here named versions and paths nobody read. A profile is
 * a command and a driver — two accounts of the same CLI are two rows, which is
 * exactly what the fixed markup could not show.
 */

export function ProviderRows(): React.JSX.Element {
  const [profiles, setProfiles] = useState<readonly Profile[]>([])
  const [checked, setChecked] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  const refresh = useCallback(() => {
    void ask(() => commands.agentProfiles()).then((answer) => {
      setProfiles(answer.data ?? [])
      setError(answer.error)
      setChecked(new Date().toLocaleTimeString())
    })
  }, [])

  useEffect(refresh, [refresh])

  return (
    <div className="card2">
      <div className="card2__top">
        <div style={{ flex: '1', minWidth: '0' }}>
          <div className="card2__t">Coding agents</div>
          <div className="card2__d">
            devpit drives agent CLIs installed on this computer. Install or sign in with each
            agent&rsquo;s own CLI, then refresh.
          </div>
        </div>
        <div>
          <button className="card2__go" onClick={refresh}>
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.9" strokeLinecap="round" strokeLinejoin="round"><path d="M21 12a9 9 0 1 1-2.6-6.4" /><path d="M21 3v6h-6" /></svg>
            Refresh
          </button>
          {checked && <div className="card2__when">Checked {checked}</div>}
        </div>
      </div>

      {error && <p className="acc__note">{error}</p>}
      {!error && profiles.length === 0 && (
        <p className="acc__note">No agent CLI found on the PATH.</p>
      )}

      {profiles.map((profile) => (
        <div className="prov" key={profile.id}>
          <span className="prov__ico">
            {profile.label.slice(0, 2).toUpperCase()}
            <span
              className="prov__dot"
              style={{ background: profile.path ? 'var(--success)' : 'var(--ghost)' }}
            ></span>
          </span>
          <span className="prov__body">
            <span className="prov__top">
              <span className="prov__n">{profile.label}</span>
              <span className="prov__v">{profile.command}</span>
            </span>
            <span className="prov__sub">
              {profile.path
                ? `${profile.path} · ${(profile.models ?? []).length} model(s)`
                : 'not on the PATH'}
            </span>
          </span>
        </div>
      ))}
    </div>
  )
}
