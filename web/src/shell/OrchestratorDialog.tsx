import { useEffect, useState } from 'react'
import { createPortal } from 'react-dom'

import type { Profile, Project } from '../gen/bindings'
import { FieldSelect } from './FieldSelect'
import { ask, commands } from './live'
import { PROFILES_CHANGED } from './profiles'
import { abandoned, committed } from './typing'
import { useShell } from './useShell'

/*
 * An orchestrator's name and the account it speaks as — made new, or, for one
 * already there, its account changed.
 *
 * The accounts are the profiles configured in Providers: each one a command
 * the person types, read once for what it sets. Claude Code only — reaching
 * other sessions is what an orchestrator is for.
 */

/** Claude Code accounts. A command devpit has not read yet is listed too, as
 *  something to set up rather than something missing. */
export const claudeAccounts = (profiles: readonly Profile[]): readonly Profile[] =>
  profiles.filter((one) => one.driver === 'claude' && one.reach !== 'missing')

export const needsReading = (profile: Profile): boolean => profile.path === null

const NEW_PROFILE = '__new__'

export function OrchestratorDialog({
  editing,
  onMade,
  onClose,
}: {
  /** An orchestrator whose account is being changed; absent when making one. */
  editing?: Project
  onMade: (made: Project | null) => void
  onClose: () => void
}): React.JSX.Element {
  const { openPrefs } = useShell()
  const [accounts, setAccounts] = useState<readonly Profile[]>([])
  const [picked, setPicked] = useState<string | null>(editing?.orchestrator ?? null)
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
  const ready = chosen !== null && !needsReading(chosen) && (editing !== undefined || name.trim() !== '') && !busy

  const save = (): void => {
    if (!ready || !chosen) return
    setBusy(true)
    const asked = editing
      ? ask(() => commands.orchestratorAccount(editing.id, chosen.id)).then((answer) => (answer.error ? setError(answer.error) : onMade(null)))
      : ask(() => commands.orchestratorCreate(chosen.id, name)).then((answer) => (answer.data ? onMade(answer.data) : setError(answer.error)))
    void asked.finally(() => setBusy(false))
  }

  return createPortal(
    <div className="ask" data-open="true" onClick={(event) => event.target === event.currentTarget && onClose()} onKeyDown={(event) => abandoned(event) && onClose()}>
      <div className="addpj__box pdlg" role="dialog" aria-modal="true" aria-labelledby="orchT">
        <div>
          <h2 className="addpj__t" id="orchT">{editing ? `${editing.name} runs as` : 'New orchestrator'}</h2>
          <p className="addpj__d">
            {editing
              ? 'Its next turn starts under this account. Conversations so far stay with the one they began with.'
              : 'A chat that sees every project and the sessions of its account.'}
          </p>
        </div>
        {!editing && (
          <label className="pdlg__f">
            <span>Name</span>
            <input autoFocus value={name} placeholder="Work" maxLength={60} onChange={(event) => setName(event.target.value)} onKeyDown={(event) => committed(event) && save()} />
          </label>
        )}
        <div className="pdlg__f">
          <span>Runs as</span>
          {accounts.length === 0 ? (
            <p className="addpj__d">No Claude Code account is configured. Add one in Settings → Providers.</p>
          ) : (
            <FieldSelect
              label="Runs as"
              value={picked}
              options={[
                ...accounts.map((one) => ({
                  id: one.id,
                  label: one.label,
                  hint: needsReading(one) ? `${one.command} · set up in Providers first` : one.command,
                })),
                { id: NEW_PROFILE, label: 'New profile…', hint: 'another command of your own' },
              ]}
              onPick={(id) => (id === NEW_PROFILE ? (onClose(), openPrefs('providers')) : setPicked(id))}
            />
          )}
          {chosen && needsReading(chosen) && (
            <p className="acc__note">
              devpit has not read what <code>{chosen.command}</code> runs yet.{' '}
              <button className="btn" onClick={() => (onClose(), openPrefs('providers'))}>
                Open it in Settings → Providers
              </button>
            </p>
          )}
        </div>
        {error && <p className="acc__note">{error}</p>}
        <div className="pdlg__acts">
          <button className="btn" onClick={onClose}>Cancel</button>
          <button className="btn btn--go" disabled={!ready} onClick={save}>{editing ? 'Save' : 'Create'}</button>
        </div>
      </div>
    </div>,
    document.querySelector('.app') ?? document.body,
  )
}
