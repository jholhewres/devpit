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
  enter: (projectId: string, minutes?: number) => void
  leave: () => void
} {
  const [focus, setFocus] = useState<HeadsDown | null>(null)

  useEffect(() => {
    void ask(() => commands.focusRead()).then((answer) => {
      if (answer.data) setFocus(answer.data)
    })
  }, [])

  const write = useCallback((projectId: string | null, minutes = 0) => {
    void ask(() => commands.focusWrite(projectId, minutes)).then((answer) => {
      /* What the app answered, not what was asked for: the second it began is
         the store's, and the pill counts from it. */
      setFocus(answer.data ?? null)
    })
  }, [])

  return {
    focus,
    enter: useCallback((projectId: string, minutes = 0) => write(projectId, minutes), [write]),
    leave: useCallback(() => write(null), [write]),
  }
}

/**
 * The focus stamped on the root element, as it is written there.
 *
 * `<project>:<second>` while the door is shut, and `<project>:<second>:open`
 * once a timebox has run out. The door is a state here rather than a time,
 * because the readers of this — the bell, the update card — have no clock of
 * their own: only the pill ticks, so only the pill decides.
 */
export const stamp = (focus: HeadsDown | null, now: number): string | null => {
  if (focus === null || focus.since === null) return null
  const open = focus.until !== null && focus.until !== undefined && now >= focus.until
  return `${focus.projectId}:${focus.since}${open ? ':open' : ''}`
}

/** What a stamp says. Anything that is not a focus reads as none. */
export function fromStamp(held: string | undefined): (HeadsDown & { open: boolean }) | null {
  const [projectId, since, door] = (held ?? '').split(':')
  if (!projectId || since === undefined) return null
  const began = Number(since)
  if (!Number.isFinite(began)) return null
  return { projectId, since: began, until: null, open: door === 'open' }
}

/**
 * The focus that is on, for anything that is not the control that owns it.
 *
 * Read off the root element rather than through `focusRead`: a second reader
 * with its own copy of the state is two answers that can disagree, and the
 * bell and the card both need this one. Watched the way `Leaf.tsx` watches
 * the theme, which is stamped in the same place.
 */
export function useFocus(): (HeadsDown & { open: boolean }) | null {
  const [focus, setFocus] = useState<(HeadsDown & { open: boolean }) | null>(() =>
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
