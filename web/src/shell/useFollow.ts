import { useCallback, useEffect, useRef, useState } from 'react'

/* How far from the end still needs no way back to it: a line or two. */
export const NEAR = 48
/* Following resumes only this close to the end, so reading the line just
   above the last one is not undone by the next token. */
export const REARM = 4
/* How far a scroll event may land from where the app sent it and still be
   the app's own: layout rounds, and the browser clamps. */
const SLACK = 2

/**
 * Whether a scroll that landed at `top` is one the app asked for. A match
 * consumes it and every older mark; nothing else can still be on its way.
 */
export function ownScroll(marks: number[], top: number): boolean {
  const at = marks.findIndex((mark) => Math.abs(mark - top) <= SLACK)
  if (at < 0) return false
  marks.splice(0, at + 1)
  return true
}

export type Follow<T> = {
  box: React.RefObject<T | null>
  /** Far enough up that the way back to the end should be offered. */
  away: boolean
  toEnd: () => void
}

/**
 * A scroller that keeps up with new output, but only while it is being
 * followed. Scrolled up to read something earlier, it stays where it was put;
 * back at the end, it follows again.
 *
 * Growth is heard as a resize, not only as `content` changing: a reply that
 * streams into the same message, or a block that opens, changes no list.
 */
export function useFollow<T extends HTMLElement>(content: unknown): Follow<T> {
  const box = useRef<T>(null)
  const following = useRef(true)
  const marks = useRef<number[]>([])
  const [away, setAway] = useState(false)

  const pin = useCallback(() => {
    const scroll = box.current
    if (!scroll || !following.current) return
    const end = Math.max(0, scroll.scrollHeight - scroll.clientHeight)
    // Already there: no event would come to consume the mark.
    if (Math.abs(scroll.scrollTop - end) <= SLACK) return
    marks.current = [...marks.current.slice(-15), end]
    scroll.scrollTop = end
  }, [])

  useEffect(() => {
    const scroll = box.current
    if (!scroll) return
    const moved = (): void => {
      const left = scroll.scrollHeight - scroll.scrollTop - scroll.clientHeight
      if (!ownScroll(marks.current, scroll.scrollTop)) {
        marks.current = []
        following.current = left <= REARM
      }
      setAway(left > NEAR)
    }
    scroll.addEventListener('scroll', moved, { passive: true })
    const sizes = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(pin)
    // The children change when the blank state gives way to a thread.
    const watch = (): void => {
      if (!sizes) return
      sizes.disconnect()
      sizes.observe(scroll)
      for (const child of Array.from(scroll.children)) sizes.observe(child)
    }
    watch()
    const kids = new MutationObserver(watch)
    kids.observe(scroll, { childList: true })
    return () => {
      scroll.removeEventListener('scroll', moved)
      sizes?.disconnect()
      kids.disconnect()
    }
  }, [pin])

  useEffect(pin, [content, pin])

  const toEnd = useCallback(() => {
    following.current = true
    setAway(false)
    pin()
  }, [pin])

  return { box, away, toEnd }
}
