import { useCallback, useEffect, useRef, useState } from 'react'

import type { Credentials, SignIns } from '../gen/bindings'
import { useAway } from './away'
import { ask, commands } from './live'
import { useShell } from './useShell'

/*
 * The sign-ins a session runs on, beside the MCP servers: each account's own
 * Claude sign-in, read from its file, and what the open project needs — an
 * AWS profile, gcloud, gh — asked of their CLIs. An expired one is seen here
 * before a session finds it halfway through a task.
 */

const EVERY_MS = 5 * 60 * 1000

export const ACCOUNT_WORDS: Record<Credentials, string> = {
  saved: 'signed in',
  expired: 'expired',
  missing: 'signed out',
  unknown: 'in the Keychain',
}

/** Those that hold out of all, and whether any needs signing in again. The
    Keychain says nothing either way, so it is not counted against. */
export function tallied(signIns: SignIns): { ok: number; all: number; bad: boolean } {
  const ok =
    signIns.accounts.filter((one) => one.state === 'saved' || one.state === 'unknown').length +
    signIns.needed.filter((one) => one.holds).length
  const all = signIns.accounts.length + signIns.needed.length
  return { ok, all, bad: ok < all }
}

export function SignInChip({ projectId }: { projectId: string | null }): React.JSX.Element | null {
  const { openPrefs } = useShell()
  const [signIns, setSignIns] = useState<SignIns | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [asking, setAsking] = useState(false)
  const [open, setOpen] = useState(false)
  const box = useRef<HTMLDivElement>(null)
  useAway(box, useCallback(() => setOpen(false), []), open)

  const check = useCallback(() => {
    setAsking(true)
    void ask(() => commands.signIns(projectId))
      .then((answer) => {
        if (answer.data) setSignIns(answer.data)
        setError(answer.error)
      })
      .finally(() => setAsking(false))
  }, [projectId])

  useEffect(() => {
    check()
    const every = window.setInterval(() => document.visibilityState === 'visible' && check(), EVERY_MS)
    return () => window.clearInterval(every)
  }, [check])

  if (!signIns && !error) return null
  if (signIns && signIns.accounts.length + signIns.needed.length === 0) return null
  const { ok, all, bad } = signIns ? tallied(signIns) : { ok: 0, all: 0, bad: true }
  return (
    <div className="mcpchip" ref={box}>
      <button className="strip__go" data-state={bad ? 'bad' : undefined} aria-expanded={open} onClick={() => setOpen((was) => !was)} title="Sign-ins">
        <span>Sign-ins</span>
        <span className="strip__n">{signIns ? `${ok}/${all}` : '?'}</span>
      </button>
      {open && (
        <div className="mcpchip__pop" role="dialog" aria-label="Sign-ins">
          <div className="mcpchip__h">
            <span>Sign-ins</span>
            <span>
              <button className="mcpchip__again" onClick={() => (setOpen(false), openPrefs('providers'))}>
                Accounts
              </button>
              <button className="mcpchip__again" disabled={asking} onClick={check} aria-label="Check the sign-ins again">
                {asking ? 'Checking…' : 'Refresh'}
              </button>
            </span>
          </div>
          {error && <p className="mcpchip__why">{error}</p>}
          <div className="mcpchip__list">
            {signIns && signIns.accounts.length > 0 && (
              <section>
                <h4 className="mcpchip__g">Accounts</h4>
                <ul>
                  {signIns.accounts.map((one) => (
                    <li key={one.directory} data-state={one.state} title={one.directory}>
                      <i className="mcpchip__dot" />
                      <span className="mcpchip__n">{one.label}</span>
                      <span className="mcpchip__s">{ACCOUNT_WORDS[one.state]}</span>
                      {(one.state === 'expired' || one.state === 'missing') && (
                        <span className="sichip__fix">
                          Run <code>{one.fix}</code>
                        </span>
                      )}
                    </li>
                  ))}
                </ul>
              </section>
            )}
            {signIns && signIns.needed.length > 0 && (
              <section>
                <h4 className="mcpchip__g">This project</h4>
                <ul>
                  {signIns.needed.map((one) => (
                    <li key={one.what} data-state={one.holds ? 'saved' : 'expired'}>
                      <i className="mcpchip__dot" />
                      <span className="mcpchip__n">{one.what}</span>
                      <span className="mcpchip__s">{one.holds ? 'signed in' : 'expired'}</span>
                      {!one.holds && (
                        <span className="sichip__fix">
                          Run <code>{one.fix}</code>
                        </span>
                      )}
                    </li>
                  ))}
                </ul>
              </section>
            )}
          </div>
        </div>
      )}
    </div>
  )
}
