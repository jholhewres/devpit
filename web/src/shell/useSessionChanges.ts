import { useCallback, useEffect, useState } from 'react'

import type { LiveSession, SessionChanges } from '../gen/bindings'
import { ask, commands } from './live'

/*
 * Where a session's folder stands, read again when the window comes back to
 * the front and whenever the session moves on — its status or its step — and
 * never on a timer.
 */

export function useSessionChanges(session: LiveSession): {
  changes: SessionChanges | null
  problem: string | null
  reload: () => void
} {
  const [changes, setChanges] = useState<SessionChanges | null>(null)
  const [problem, setProblem] = useState<string | null>(null)
  const { cwd, cardId } = session

  const reload = useCallback(() => {
    void ask(() => commands.sessionChanges(cwd, cardId)).then((answer) => {
      setProblem(answer.error)
      if (answer.data) setChanges(answer.data)
    })
  }, [cwd, cardId])

  useEffect(reload, [reload, session.status, session.step])

  useEffect(() => {
    window.addEventListener('focus', reload)
    return () => window.removeEventListener('focus', reload)
  }, [reload])

  return { changes, problem, reload }
}
