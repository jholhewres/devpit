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

/**
 * Whether a focus is on, for something that only needs to know that much.
 *
 * Read off the root element rather than through `focusRead`: the card that
 * asks this is not the one that owns the focus, and a second reader with its
 * own copy of the state is two answers that can disagree. Watched the way
 * `Leaf.tsx` watches the theme, which is stamped in the same place.
 */
export function useFocusIsOn(): boolean {
  const [on, setOn] = useState(
    () => typeof document !== 'undefined' && document.documentElement.dataset.headsDown === 'true',
  )

  useEffect(() => {
    const root = document.documentElement
    const look = (): void => setOn(root.dataset.headsDown === 'true')
    look()
    const watching = new MutationObserver(look)
    watching.observe(root, { attributes: true, attributeFilter: ['data-heads-down'] })
    return () => watching.disconnect()
  }, [])

  return on
}
