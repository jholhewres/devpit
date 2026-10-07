import { useCallback, useEffect, useRef, useState } from 'react'

import type { AgentHealth } from '../gen/bindings'
import { useAway } from './away'
import { ask, commands } from './live'
import { onAgentHealth } from './window'

/*
 * devpit's own MCP — the door its tools post to — beside the account's
 * servers: whether it answers, and a restart that leaves the window, the
 * chats and the terminals where they are.
 */

/** What the chip says: the answer's time, or that there was none. */
export function agentWord(health: AgentHealth): string {
  if (!health.answering) return 'no answer'
  return health.latencyMs === null ? 'ok' : `${Math.round(health.latencyMs)} ms`
}

export function AgentChip(): React.JSX.Element | null {
  const [health, setHealth] = useState<AgentHealth | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [open, setOpen] = useState(false)
  const box = useRef<HTMLDivElement>(null)
  useAway(box, useCallback(() => setOpen(false), []), open)

  const answered = (answer: { data: AgentHealth | null; error: string | null }): void => {
    if (answer.data) setHealth(answer.data)
    setError(answer.error)
  }

  useEffect(() => {
    void ask(() => commands.agentHealth()).then(answered)
    return onAgentHealth(setHealth)
  }, [])

  const restart = (): void => {
    setBusy(true)
    void ask(() => commands.agentRestart())
      .then(answered)
      .finally(() => setBusy(false))
  }

  if (!health && !error) return null
  const bad = Boolean(error) || (health !== null && !health.answering)
  return (
    <div className="mcpchip" ref={box}>
      <button className="strip__go" data-state={bad ? 'bad' : undefined} aria-expanded={open} onClick={() => setOpen((was) => !was)} title="devpit's own MCP">
        <span>devpit</span>
        <span className="strip__n">{health ? agentWord(health) : '?'}</span>
      </button>
      {open && (
        <div className="mcpchip__pop" role="dialog" aria-label="devpit's MCP">
          <div className="mcpchip__h">
            <span>devpit&rsquo;s MCP</span>
            <button className="mcpchip__again" disabled={busy} onClick={restart} title="Opens its tools again; the window, chats and terminals stay">
              {busy ? 'Restarting…' : 'Restart MCP'}
            </button>
          </div>
          {error && <p className="mcpchip__why">{error}</p>}
          {health && (
            <p className="agentchip__said">
              {health.answering ? `Answering, in ${agentWord(health)}.` : `Not answering: ${health.failures} check${health.failures === 1 ? '' : 's'} in a row.`}
              {health.restarts > 0 && ` Restarted ${health.restarts} time${health.restarts === 1 ? '' : 's'}.`}
              {health.detail && <span className="mcpchip__why">{health.detail}</span>}
            </p>
          )}
        </div>
      )}
    </div>
  )
}
