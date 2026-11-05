import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react'

import type { Message } from '../gen/bindings'

/*
 * The messages of a long conversation drawn as a window over it: what is in
 * view and a margin each side, with the rest standing in as two spacers of
 * the height it had, or is guessed to have. A thread of thousands of
 * messages then costs the DOM of a few dozen.
 */

/* The thread's gap between rows, which a row's height carries. */
export const GAP = 20
/* How far past each edge of the view is drawn. */
export const OVER = 600
/* Rows drawn at most, whatever the view. */
export const MOST = 60
/* Rows a conversation opens with, before anything is measured. */
export const FIRST = 16

/* A row's height before it was ever drawn, by what it holds. */
export function estimate(message: Message): number {
  const text = message.parts.reduce((sum, part) => sum + ('text' in part ? part.text.length : 0), 0)
  if (message.role === 'user') return 48 + 22 * Math.ceil(text / 90) + GAP
  const acts = message.parts.filter((part) => part.kind !== 'text').length
  return 64 + 28 * Math.min(acts, 6) + 22 * Math.ceil(text / 110) + GAP
}

/* The rows that cover `over` past each edge of the view, `top` being where it starts. */
export function spanAt(heights: readonly number[], top: number, view: number, over: number): { from: number; to: number } {
  let y = 0
  let from = 0
  while (from < heights.length - 1 && y + heights[from]! <= top - over) y += heights[from++]!
  let to = from
  while (to < heights.length && y < top + view + over) y += heights[to++]!
  to = Math.max(to, Math.min(heights.length, from + 1))
  return { from: Math.max(from, to - MOST), to }
}

/* Which rows are drawn: the last `count` while the end is in reach, else
   from the message `first` on, so a message arriving at the end while the
   person reads higher up moves nothing on screen. */
type Span = { readonly tail: boolean; readonly first: string | null; readonly count: number }

export type Windowed = {
  readonly from: number
  readonly to: number
  /** Height of what is above the drawn rows, and below them. */
  readonly before: number
  readonly after: number
  /** Draws the message `id`, once it is among the messages, and brings it into view. */
  readonly reveal: (id: string, then?: (row: HTMLElement) => void) => void
}

/* The rows drawn to show message `at`: a couple above it, and enough below. */
export function spanAround(messages: readonly Message[], at: number): Span {
  const from = Math.max(0, at - 2)
  const count = Math.max(FIRST, at - from + 1)
  const tail = from + count >= messages.length
  return { tail, first: tail ? null : (messages[from]?.id ?? null), count }
}

export function rangeOf(messages: readonly Message[], span: Span): { from: number; to: number } {
  const at = span.tail || span.first === null ? -1 : messages.findIndex((one) => one.id === span.first)
  if (at < 0) return { from: Math.max(0, messages.length - span.count), to: messages.length }
  return { from: at, to: Math.min(messages.length, at + span.count) }
}

export function useWindow(messages: readonly Message[], box: React.RefObject<HTMLElement | null>): Windowed {
  const sizes = useRef(new Map<string, number>())
  const [span, setSpan] = useState<Span>({ tail: true, first: null, count: FIRST })
  const latest = useRef({ messages, span })
  latest.current = { messages, span }
  /* A row on screen and where it sat, kept across a move that draws rows above it. */
  const anchor = useRef<{ id: string; at: number } | null>(null)
  /* A message asked to be shown, waiting for its page or its draw. */
  const wanted = useRef<{ id: string; then?: (row: HTMLElement) => void; moved: number } | null>(null)
  const [, nudge] = useState(0)
  const reveal = useCallback((id: string, then?: (row: HTMLElement) => void) => {
    wanted.current = { id, then, moved: 0 }
    nudge((n) => n + 1)
  }, [])

  const height = useCallback((one: Message) => sizes.current.get(one.id) ?? estimate(one), [])
  const { from, to } = rangeOf(messages, span)
  let before = 0
  for (let at = 0; at < from; at += 1) before += height(messages[at]!)
  let after = 0
  for (let at = to; at < messages.length; at += 1) after += height(messages[at]!)

  const look = useCallback(() => {
    const scroll = box.current
    /* Not laid out (a hidden tab): nothing to measure the view against. */
    if (!scroll || scroll.clientHeight === 0) return
    const { messages: list, span: now } = latest.current
    const heights = list.map(height)
    const drawn = rangeOf(list, now)
    const need = spanAt(heights, scroll.scrollTop, scroll.clientHeight, OVER / 2)
    if (drawn.from <= need.from && drawn.to >= need.to) return
    const next = spanAt(heights, scroll.scrollTop, scroll.clientHeight, OVER)
    if (next.from < drawn.from) {
      const top = scroll.getBoundingClientRect().top
      const row = Array.from(scroll.querySelectorAll<HTMLElement>('.trow')).find((one) => one.getBoundingClientRect().bottom > top)
      anchor.current = row?.dataset.id ? { id: row.dataset.id, at: row.getBoundingClientRect().top - top } : null
    }
    const tail = next.to >= list.length
    setSpan({ tail, first: tail ? null : (list[next.from]?.id ?? null), count: next.to - next.from })
  }, [box, height])

  /* Measured after every draw, and the row read before a move put back where it was. */
  useLayoutEffect(() => {
    const scroll = box.current
    if (!scroll) return
    for (const row of Array.from(scroll.querySelectorAll<HTMLElement>('.trow'))) {
      if (row.dataset.id && row.offsetHeight > 0) sizes.current.set(row.dataset.id, row.offsetHeight + GAP)
    }
    const want = wanted.current
    if (want) {
      const row = Array.from(scroll.querySelectorAll<HTMLElement>('.trow')).find((one) => one.dataset.id === want.id)
      const at = latest.current.messages.findIndex((one) => one.id === want.id)
      if (row) {
        wanted.current = null
        anchor.current = null
        scroll.scrollTop += row.getBoundingClientRect().top - scroll.getBoundingClientRect().top - Math.max(0, (scroll.clientHeight - row.offsetHeight) / 3)
        want.then?.(row)
        return
      }
      // Drawn on the next render; a page still on its way is waited for, a few moves at most.
      if (at >= 0 && want.moved < 3) {
        want.moved += 1
        setSpan(spanAround(latest.current.messages, at))
        return
      }
      if (at >= 0) wanted.current = null
    }
    const kept = anchor.current
    anchor.current = null
    if (!kept) return
    const row = Array.from(scroll.querySelectorAll<HTMLElement>('.trow')).find((one) => one.dataset.id === kept.id)
    if (row) scroll.scrollTop += row.getBoundingClientRect().top - scroll.getBoundingClientRect().top - kept.at
  })

  useEffect(() => {
    const scroll = box.current
    if (!scroll) return
    scroll.addEventListener('scroll', look, { passive: true })
    return () => scroll.removeEventListener('scroll', look)
  }, [box, look])

  /* A page above, or a reply growing, can leave the view short of rows. */
  useEffect(look, [messages.length, look])

  return { from, to, before, after, reveal }
}
