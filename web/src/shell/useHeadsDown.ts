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

/** The focus stamped on the root element, as it is written there. */
export const stamp = (focus: HeadsDown | null): string | null =>
  focus === null || focus.since === null ? null : `${focus.projectId}:${focus.since}`

/** And back. Anything that is not a focus reads as none. */
export function fromStamp(held: string | undefined): HeadsDown | null {
  const [projectId, since] = (held ?? '').split(':')
  if (!projectId || since === undefined) return null
  const began = Number(since)
  return Number.isFinite(began) ? { projectId, since: began } : null
}

/**
 * The focus that is on, for anything that is not the control that owns it.
 *
 * Read off the root element rather than through `focusRead`: a second reader
 * with its own copy of the state is two answers that can disagree, and the
 * bell and the card both need this one. Watched the way `Leaf.tsx` watches
 * the theme, which is stamped in the same place.
 */
export function useFocus(): HeadsDown | null {
  const [focus, setFocus] = useState<HeadsDown | null>(() =>
    typeof document === 'undefined' ? null : fromStamp(document.documentElement.dataset.headsDown),
  )

  useEffect(() => {
    const root = document.documentElement
    const look = (): void => setFocus(fromStamp(root.dataset.headsDown))
    look()
    const watching = new MutationObserver(look)
    watching.observe(root, { attributes: true, attributeFilter: ['data-heads-down'] })
    return () => watching.disconnect()
  }, [])

  return focus
}

/** Whether a focus is on, for something that only needs to know that much. */
export const useFocusIsOn = (): boolean => useFocus() !== null
