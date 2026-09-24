import { useEffect, useState } from 'react'
import { createPortal } from 'react-dom'

import type { Profile, Project } from '../gen/bindings'
import { ask, commands } from './live'
import { PROFILES_CHANGED } from './profiles'
import { abandoned, committed } from './typing'
import { useShell } from './useShell'

/*
 * A new orchestrator: which account it speaks as, by the command that starts
 * it, and a name for it. The same accounts the chat offers, Claude Code only —
 * reaching other sessions is what one is for.
 */

/** Claude Code accounts. A shell function devpit has not read yet is listed
 *  too, as something to set up rather than something missing. */
export const claudeAccounts = (profiles: readonly Profile[]): readonly Profile[] =>
  profiles.filter((one) => one.driver === 'claude' && one.reach !== 'missing')

export const needsReading = (profile: Profile): boolean => profile.path === null

export function OrchestratorDialog({ onMade, onClose }: { onMade: (made: Project) => void; onClose: () => void }): React.JSX.Element {
  const { openPrefs } = useShell()
  const [accounts, setAccounts] = useState<readonly Profile[]>([])
  const [picked, setPicked] = useState<string | null>(null)
  const [name, setName] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  useEffect(() => {
    const read = (): void =>
      void ask(() => commands.agentProfiles()).then((found) => {
        const listed = claudeAccounts(found.data ?? [])
        setAccounts(listed)
        setPicked((was) => was ?? listed.find((one) => !needsReading(one))?.id ?? null)
      })
    read()
    window.addEventListener(PROFILES_CHANGED, read)
    return () => window.removeEventListener(PROFILES_CHANGED, read)
  }, [])

  const chosen = accounts.find((one) => one.id === picked) ?? null
  const ready = chosen !== null && !needsReading(chosen) && name.trim() !== '' && !busy

  const make = (): void => {
    if (!ready || !chosen) return
    setBusy(true)
    void ask(() => commands.orchestratorCreate(chosen.id, name))
      .then((answer) => (answer.data ? onMade(answer.data) : setError(answer.error)))
      .finally(() => setBusy(false))
  }

  return createPortal(
    <div className="ask" data-open="true" onClick={(event) => event.target === event.currentTarget && onClose()} onKeyDown={(event) => abandoned(event) && onClose()}>
      <div className="addpj__box pdlg" role="dialog" aria-modal="true" aria-labelledby="orchT">
        <div>
          <h2 className="addpj__t" id="orchT">New orchestrator</h2>
          <p className="addpj__d">A chat that sees every project and the sessions of its account.</p>
        </div>
        <label className="pdlg__f">
          <span>Name</span>
          <input autoFocus value={name} placeholder="Work" maxLength={60} onChange={(event) => setName(event.target.value)} onKeyDown={(event) => committed(event) && make()} />
        </label>
        <div className="pdlg__f">
          <span>Runs as</span>
          <div className="orchdlg__accounts" role="radiogroup">
            {accounts.length === 0 && <p className="addpj__d">No Claude Code account is installed.</p>}
            {accounts.map((one) => (
              <button key={one.id} className="orchdlg__acct" role="radio" aria-checked={picked === one.id} onClick={() => setPicked(one.id)}>
                <code>{one.command}</code>
                <span>{needsReading(one) ? 'set up first' : one.label}</span>
              </button>
            ))}
          </div>
          {chosen && needsReading(chosen) && (
            <p className="acc__note">
              <code>{chosen.command}</code> is a shell function, and devpit has not read what it runs yet.{' '}
              <button className="btn" onClick={() => (onClose(), openPrefs('providers'))}>Open it in Settings → Providers</button>
            </p>
          )}
        </div>
        {error && <p className="acc__note">{error}</p>}
        <div className="pdlg__acts">
          <button className="btn" onClick={onClose}>Cancel</button>
          <button className="btn btn--go" disabled={!ready} onClick={make}>Create</button>
        </div>
      </div>
    </div>,
    document.querySelector('.app') ?? document.body,
  )
}
