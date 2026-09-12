import { useEffect, type RefObject } from 'react'
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
  useEffect(() => {
    if (!open) return
    const away = (event: MouseEvent): void => {
      if (!box.current?.contains(event.target as Node)) close()
    }
    const key = (event: KeyboardEvent): void => {
      if (abandoned(event)) close()
    }
    window.addEventListener('mousedown', away)
    window.addEventListener('keydown', key)
    return () => {
      window.removeEventListener('mousedown', away)
      window.removeEventListener('keydown', key)
    }
  }, [box, close, open])
}
