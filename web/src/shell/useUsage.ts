import { useCallback, useState } from 'react'

import type { Usage } from '../gen/bindings'
import { ask, commands } from './live'
import { whileWatched } from './whileWatched'

/*
 * What the terminals are costing, on a timer.
 *
 * Three seconds and not one: reading it walks the page tables of every process
 * under every pane, which is the expensive half of the answer, and a number
 * that moves every second is a number nobody can read anyway.
 *
 * Only while somebody is looking. The strip is at the bottom of the window and
 * the monitor is a panel over it — asking when neither is on screen is a
 * kernel walk a few hundred processes deep for nobody.
 */

const EVERY = 3000

const NOTHING: Usage = { memoryKb: 0, cpuTenths: 0, proportional: true, panes: [] }

export function useUsage(projectId: string | null, watching: boolean): Usage {
  const [usage, setUsage] = useState<Usage>(NOTHING)

  /* The same numbers every three seconds are not a new state, and a new state
     is a render of the strip and what reads it. */
  const look = useCallback(() => {
    if (!projectId) return setUsage(NOTHING)
    void ask(() => commands.sessionUsage(projectId)).then((asked) => {
      const next = asked.data
      if (next) setUsage((was) => (JSON.stringify(was) === JSON.stringify(next) ? was : next))
    })
  }, [projectId])

  whileWatched(look, EVERY, projectId, watching)

  return usage
}
