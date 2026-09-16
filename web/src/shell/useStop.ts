import { useEffect, useRef, useState } from 'react'

import { isArmed, pressed, same, stops, type Armed } from './stop'

/*
 * Escape, twice, stops the turn — wired to the window.
 *
 * The window and not the textarea: your hands may be anywhere on the pane
 * when you decide to stop it, and a shortcut that only works while one field
 * has focus is a shortcut you have to think about.
 *
 * The rules are in `stop.ts`; this is only the listener and the timer that
 * lets an arming lapse.
 */

/** What takes Escape for itself while it is open. `:not([hidden])` because the
    sidebar keeps its menus mounted and hidden, and counting those meant the
    double Escape never armed the stop at all. */
export const OWNS_ESCAPE = '[role="menu"]:not([hidden]), [role="dialog"]:not([hidden])'

export function useStop(
  active: boolean,
  target: string,
  onStop: () => void,
): boolean {
  const [armed, setArmed] = useState<Armed | null>(null)
  const timer = useRef<number | null>(null)

  useEffect(() => {
    if (!active) {
      setArmed(null)
      return
    }
    const onKey = (event: KeyboardEvent): void => {
      /* A menu or a dialog owns Escape while it is open: closing it is what
         the person meant, and stopping the turn behind it is not. */
      if (!stops(event, document.querySelector(OWNS_ESCAPE) !== null)) return
      event.preventDefault()
      const press = pressed(armed, target, Date.now())
      if (press.type === 'stop') {
        setArmed(null)
        onStop()
        return
      }
      setArmed(press.armed)
      if (timer.current !== null) window.clearTimeout(timer.current)
      timer.current = window.setTimeout(
        () => setArmed((was) => (same(was, press.armed) ? null : was)),
        Math.max(0, press.armed.until - Date.now()),
      )
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [active, armed, onStop, target])

  useEffect(() => () => {
    if (timer.current !== null) window.clearTimeout(timer.current)
  }, [])

  return active && isArmed(armed, target, Date.now())
}
