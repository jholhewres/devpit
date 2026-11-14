import { useEffect, useState } from 'react'

import { ask, commands } from './live'

/*
 * How often the footer's sign-in and MCP chips look again: Settings →
 * Providers chooses it, and a choice reaches the chips already on screen.
 */

export const CHECK_MINUTES = [1, 5, 15, 30] as const
/** What the chips always waited. */
export const CHECK_DEFAULT = 5
const CHANGED = 'devpit:provider-checks'

/** Tells the chips on screen the minutes just chosen. */
export function checksChanged(minutes: number): void {
  window.dispatchEvent(new CustomEvent<number>(CHANGED, { detail: minutes }))
}

/** Milliseconds between two looks. */
export function useCheckEvery(): number {
  const [minutes, setMinutes] = useState(CHECK_DEFAULT)
  useEffect(() => {
    void ask(() => commands.providersCheckRead()).then((answer) => answer.data && setMinutes(answer.data.minutes))
    const heard = (event: Event): void => setMinutes((event as CustomEvent<number>).detail)
    window.addEventListener(CHANGED, heard)
    return () => window.removeEventListener(CHANGED, heard)
  }, [])
  return minutes * 60 * 1000
}
