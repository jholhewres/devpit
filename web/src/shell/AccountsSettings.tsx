import { useCallback, useEffect, useState } from 'react'

import type { AccountQuota, Accounts } from '../gen/bindings'
import { accountOf, SAID, withDefault, without } from './accounts'
import { ask, commands } from './live'
import { resetIn, n } from './usageFormat'
import { useShell } from './useShell'

/*
 * Settings → Accounts: the Claude Code accounts devpit opens sessions in,
 * each a name and the config directory its sign-in lives in, with how much of
 * its plan is left and the account this project's new sessions use.
 *
 * Choosing one changes the next session, never one that runs.
 */

function Quota({ quota, now }: { quota: AccountQuota | undefined; now: number }): React.JSX.Element {
  if (!quota) return <span className="pref__d">…</span>
  if (quota.state !== 'read') return <span className="pref__d">{SAID[quota.state]}</span>
  return (
    <div className="use__limits">
      {quota.windows.map((window) => (
        <div className="bar" key={window.label}>
          <div className="bar__top">
            <span className="bar__n">{window.label}</span>
            <span className="bar__v">{Math.round(n(window.percent))}%</span>
          </div>
          <div className="bar__track">
            <div className="bar__fill" data-high={n(window.percent) >= 80} style={{ width: `${n(window.percent)}%` }} />
          </div>
          {window.resetsAt !== null && <div className="bar__sub">{resetIn(window.resetsAt, now)}</div>}
        </div>
      ))}
    </div>
  )
}

export function AccountsSettings(): React.JSX.Element {
  const { project } = useShell()
  const [accounts, setAccounts] = useState<Accounts>({ accounts: [], projects: [] })
  const [quotas, setQuotas] = useState<readonly AccountQuota[]>([])
  const [name, setName] = useState('')
  const [dir, setDir] = useState('')
  const [said, setSaid] = useState<string | null>(null)

  const readQuota = useCallback(() => {
    void ask(() => commands.accountsQuota()).then((answer) => setQuotas(answer.data?.accounts ?? []))
  }, [])

  useEffect(() => {
    void ask(() => commands.accountsRead()).then((answer) => answer.data && setAccounts(answer.data))
    readQuota()
  }, [readQuota])

  const save = (next: Accounts): void => {
    void ask(() => commands.accountsSave(next)).then((answer) => {
      setSaid(answer.error)
      if (!answer.data) return
      setAccounts(answer.data)
      readQuota()
    })
  }

  const add = (): void => {
    save({ ...accounts, accounts: [...accounts.accounts, { name: name.trim(), configDir: dir.trim() }] })
    setName('')
    setDir('')
  }

  const current = project ? accountOf(accounts, project.id) : null
  const now = Date.now()

  return (
    <>
      <h1 className="prefs__h">Accounts</h1>
      <p className="acc__note">
        Each account is a Claude Code config directory with its own sign-in. Sessions devpit starts open with it as
        CLAUDE_CONFIG_DIR, talking to Anthropic directly, so Remote Control keeps working.
      </p>

      {accounts.accounts.map((one) => (
        <div className="pref pref--stack" key={one.name}>
          <div className="pref__body">
            <span className="pref__t">{one.name}</span>
            <span className="pref__d">
              <code>{one.configDir}</code>
            </span>
            <Quota quota={quotas.find((quota) => quota.name === one.name)} now={now} />
            <span className="voice__row">
              <button className="btn" aria-label={`Remove ${one.name}`} onClick={() => save(without(accounts, one.name))}>
                Remove
              </button>
            </span>
          </div>
        </div>
      ))}

      <div className="pref pref--stack">
        <div className="pref__body">
          <span className="pref__t">Add an account</span>
          <label className="voice__row">
            <span className="voice__l">Name</span>
            <input className="voice__in" aria-label="Account name" value={name} onChange={(event) => setName(event.target.value)} />
          </label>
          <label className="voice__row">
            <span className="voice__l">Config directory</span>
            <input className="voice__in" aria-label="Account config directory" placeholder="~/.claude-work" spellCheck={false} value={dir} onChange={(event) => setDir(event.target.value)} />
          </label>
          <span className="voice__row">
            <button className="btn" disabled={!name.trim() || !dir.trim()} onClick={add}>
              Add
            </button>
          </span>
          {said && <span className="pref__d voice__bad">{said}</span>}
        </div>
      </div>

      {project && accounts.accounts.length > 0 && (
        <label className="voice__row">
          <span className="voice__l">{project.name} opens in</span>
          <select
            className="use__pick"
            aria-label={`Account for ${project.name}`}
            value={current ?? ''}
            onChange={(event) => save(withDefault(accounts, project.id, event.target.value || null))}
          >
            <option value="">The profile&rsquo;s own</option>
            {accounts.accounts.map((one) => (
              <option key={one.name} value={one.name}>
                {one.name}
              </option>
            ))}
          </select>
        </label>
      )}
    </>
  )
}
