import { useEffect, useState } from 'react'

import type { Happening } from '../gen/bindings'
import { onHappening } from './window'

/*
 * Which CLI session the agent in each pane is in, from the agent's own hooks.
 *
 * Known only for agents this app started, because only those carry the hooks.
 * It is what lets a terminal's conversation go on in a chat.
 */

export type PaneSession = { readonly sessionId: string; readonly installation: string }

export type PaneSessions = Readonly<Record<string, PaneSession>>

/* `<installation>/projects/<folder>/<session>.jsonl`. The folder name has its
   slashes replaced, so the last `/projects/` is the installation's own. */
export function installationOf(transcript: string): string | null {
  const at = transcript.lastIndexOf('/projects/')
  return at > 0 ? transcript.slice(0, at) : null
}

export function afterSession(held: PaneSessions, paneId: string, detail: string | null): PaneSessions {
  if (!detail) return held
  let read: unknown
  try {
    read = JSON.parse(detail)
  } catch {
    return held
  }
  const { sessionId, transcript } = (read ?? {}) as { sessionId?: unknown; transcript?: unknown }
  if (typeof sessionId !== 'string' || !sessionId || typeof transcript !== 'string') return held
  const installation = installationOf(transcript)
  if (!installation) return held
  const was = held[paneId]
  if (was?.sessionId === sessionId && was.installation === installation) return held
  return { ...held, [paneId]: { sessionId, installation } }
}

export function usePaneSessions(): PaneSessions {
  const [held, setHeld] = useState<PaneSessions>({})

  useEffect(
    () =>
      onHappening((happening: Happening) => {
        if (happening.what !== 'session') return
        setHeld((was) => afterSession(was, happening.paneId, happening.detail))
      }),
    [],
  )

  return held
}
