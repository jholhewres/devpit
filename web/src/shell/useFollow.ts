import { useEffect, useRef } from 'react'

/* How near the bottom still counts as at it, in pixels: a line or two, so a
   stray wheel tick does not stop the follow. */
const NEAR = 48

/**
 * A scroller that keeps up with new output, but only while it is being
 * followed. Scrolled up to read something earlier, it stays where it was put;
 * back at the bottom, it follows again.
 */
export function useFollow<T extends HTMLElement>(content: unknown): React.RefObject<T | null> {
  const box = useRef<T>(null)
  const following = useRef(true)

  useEffect(() => {
    const scroll = box.current
    if (!scroll) return
    const moved = (): void => {
      following.current = scroll.scrollHeight - scroll.scrollTop - scroll.clientHeight < NEAR
    }
    scroll.addEventListener('scroll', moved, { passive: true })
    return () => scroll.removeEventListener('scroll', moved)
  }, [])

  useEffect(() => {
    const scroll = box.current
    if (scroll && following.current) scroll.scrollTop = scroll.scrollHeight
  }, [content])

  return box
}
