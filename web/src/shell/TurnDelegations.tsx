import { useContext, useState } from 'react'

import type { LiveSession } from '../gen/bindings'

import type { Part } from '../gen/bindings'
import { STATE_WORDS, stateOf, useLiveSessions } from './liveStatus'
import { AppsScopeContext } from './mcpApps'
import { SessionTerminal } from './SessionTerminal'
import { useShell } from './useShell'

/*
 * What a turn handed to other sessions, as cards: to whom, what was asked,
 * and how that session stands now. A message sent is work somebody else is
 * doing, and the row it used to be folded into said none of that.
 */
export function TurnDelegations({ parts }: { parts: readonly Part[] }): React.JSX.Element | null {
  const scope = useContext(AppsScopeContext)
  const sent = parts.flatMap((part) => {
    if (part.kind !== 'tool_call' || part.parent || part.name !== 'SendMessage') return []
    try {
      const input = JSON.parse(part.input) as { to?: string; summary?: string; message?: string }
      return input.to ? [{ id: part.id, to: input.to, summary: input.summary ?? '', message: input.message ?? '' }] : []
    } catch {
      return []
    }
  })
  const sessions = useLiveSessions(sent.length > 0 ? (scope?.profileId ?? null) : null)
  if (sent.length === 0) return null

  return (
    <div className="deleg">
      {sent.map((one) => (
        <Delegation key={one.id} to={one.to} summary={one.summary} message={one.message} state={stateOf(sessions, one.to)} session={sessions.find((live) => live.name === one.to) ?? null} />
      ))}
    </div>
  )
}

function Delegation({ to, summary, message, state, session }: { to: string; summary: string; message: string; state: keyof typeof STATE_WORDS; session: LiveSession | null }): React.JSX.Element {
  const [open, setOpen] = useState(false)
  const [terminal, setTerminal] = useState(false)
  const { setProject, openPane } = useShell()
  return (
    <div className="deleg__card" data-state={state}>
      <button className="deleg__h" aria-expanded={open} onClick={() => setOpen((was) => !was)}>
        <span className="deleg__to">→ {to}</span>
        <span className="deleg__what">{summary || 'a message'}</span>
        <span className="deleg__state">
          <i className="deleg__dot" />
          {STATE_WORDS[state]}
        </span>
      </button>
      {open && <p className="deleg__msg">{message}</p>}
      {open && session?.pane && (
        <div className="deleg__acts">
          <button className="btn" onClick={() => setTerminal(true)}>Open its terminal</button>
        </div>
      )}
      {terminal && session && (
        <SessionTerminal
          session={session}
          onClose={() => setTerminal(false)}
          onGo={() => {
            setTerminal(false)
            if (session.projectId) setProject(session.projectId)
            if (session.pane) openPane(session.pane.paneId)
          }}
        />
      )}
    </div>
  )
}
