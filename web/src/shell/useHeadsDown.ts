import { useCallback, useEffect, useState } from 'react'

import { ask, commands } from './live'
import type { HeadsDown } from '../gen/bindings'

/*
 * Going into a focus and coming out of one.
 *
 * The state lives in the store, not here: the elapsed time on screen is
 * counted from the second it began, so it has to survive a restart. This hook
 * reads it once and keeps what the app answered.
 */

export function useHeadsDown(): {
  focus: HeadsDown | null
  enter: (projectId: string) => void
  leave: () => void
} {
  const [focus, setFocus] = useState<HeadsDown | null>(null)

  useEffect(() => {
    void ask(() => commands.focusRead()).then((answer) => {
      if (answer.data) setFocus(answer.data)
    })
  }, [])

  const write = useCallback((projectId: string | null) => {
    void ask(() => commands.focusWrite(projectId)).then((answer) => {
      /* What the app answered, not what was asked for: the second it began is
         the store's, and the pill counts from it. */
      setFocus(answer.data ?? null)
    })
  }, [])

  return {
    focus,
    enter: useCallback((projectId: string) => write(projectId), [write]),
    leave: useCallback(() => write(null), [write]),
  }
}
