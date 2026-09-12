import { useEffect, useState } from 'react'

import type { Happening } from '../gen/bindings'
import { onHappening } from './window'

/*
 * What each agent says it is doing.
 *
 * A different question from `useRunning`, and it needs a different answer.
 * The process table can say *which* agent is open in a pane — it reads the
 * argument vector, works for every CLI, and needs nobody's permission. What it
 * cannot say is whether that agent is working or has stopped and is waiting
 * for a person, because an agent blocked on the network and an agent blocked
 * on you look identical from outside.
 *
 * Only the agent knows, so the agent is asked: its hooks post to a loopback
 * listener, carrying the pane they fired in. Pushed rather than polled,
 * because it changes at the moment it changes and a timer would find out late.
 *
 * This only ever describes agents this app started, which is the set it has
 * any business watching — the hooks travel on the command line, not in the
 * person's own settings file.
 */

export type AgentState = 'working' | 'waiting' | 'done'

const STATES: readonly string[] = ['working', 'waiting', 'done']

/** What each pane's agent last reported, by pane. */
export type Doing = Readonly<Record<string, AgentState>>

export function useAgents(): Doing {
  const [doing, setDoing] = useState<Doing>({})

  useEffect(
    () =>
      onHappening((happening: Happening) => {
        if (happening.what !== 'agent') return
        const state = happening.detail
        if (!state || !STATES.includes(state)) return
        setDoing((was) =>
          was[happening.paneId] === state
            ? was
            : { ...was, [happening.paneId]: state as AgentState },
        )
      }),
    [],
  )

  return doing
}
