import { useEffect, useRef, type RefObject } from 'react'

import { claimMenu } from './menuRules'
import { abandoned } from './typing'

/* Closing a menu by clicking off it, and by Escape.
 *
 * Three menus wanted this and each had written its own; the third was where
 * they started to disagree about whether Escape counted. */
export function useAway(
  box: RefObject<HTMLElement | null>,
  close: () => void,
  open: boolean,
): void {
  /* The latest close, read when it is needed: callers write it inline, and
     an effect keyed on it would give the one-menu slot up and take it back —
     closing itself — on every render. */
  const closing = useRef(close)
  closing.current = close
  useEffect(() => {
    if (!open) return
    const close = (): void => closing.current()
    /* One menu at a time with every other (`claimMenu`), and closed by a
       right-click elsewhere too — in capture, so a handler that stops the
       event on its way to opening its own menu does not keep this one open. */
    const release = claimMenu(close)
    const away = (event: Event): void => {
      if (!box.current?.contains(event.target as Node)) close()
    }
    const key = (event: KeyboardEvent): void => {
      if (abandoned(event)) close()
    }
    window.addEventListener('mousedown', away, true)
    window.addEventListener('contextmenu', away, true)
    window.addEventListener('keydown', key)
    return () => {
      release()
      window.removeEventListener('mousedown', away, true)
      window.removeEventListener('contextmenu', away, true)
      window.removeEventListener('keydown', key)
    }
  }, [box, open])
}
