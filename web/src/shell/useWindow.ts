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
export function estimate(message: Message | undefined): number {
  if (!message) return GAP
  const text = message.parts.reduce((sum, part) => sum + ('text' in part ? part.text.length : 0), 0)
  const acts = message.parts.filter((part) => part.kind !== 'text').length
  return message.role === 'user' ? 48 + 22 * Math.ceil(text / 90) + GAP : 64 + 28 * Math.min(acts, 6) + 22 * Math.ceil(text / 110) + GAP
}

/* The two spacers, carried from the draw before. */
export type Spacers = {
  readonly ids: readonly string[]
  readonly from: number
  readonly to: number
  /** Height of the rows above `from`, and from `to` on. */
  readonly before: number
  readonly after: number
}

const extends_ = (ids: readonly string[], was: readonly string[]): boolean =>
  ids === was || (was.length <= ids.length && was.every((id, at) => ids[at] === id))

/* The spacers for rows `from`..`to` of `ids`, from the last ones: a window
   that moved adds and takes off only the rows that crossed its edges, and
   messages added at the end only themselves. Anything else is summed again.
   It holds because a row out of view is never measured: its height is the
   one it left with. */
export function spacers(was: Spacers | null, ids: readonly string[], from: number, to: number, height: (at: number) => number): Spacers {
  const sum = (start: number, end: number): number => {
    let total = 0
    for (let at = start; at < end; at += 1) total += height(at)
    return total
  }
  if (!was || !extends_(ids, was.ids)) return { ids, from, to, before: sum(0, from), after: sum(to, ids.length) }
  const before = was.before + (from >= was.from ? sum(was.from, from) : -sum(from, was.from))
  const after = was.after + sum(was.ids.length, ids.length) + (to >= was.to ? -sum(was.to, to) : sum(to, was.to))
  return { ids, from, to, before, after }
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
  /** A row drawn again on its own says how tall it is now. */
  readonly measure: (id: string, row: HTMLElement) => void
}

/* The rows drawn to show message `at`: a couple above it, and enough below. */
export function spanAround(ids: readonly string[], at: number): Span {
  const from = Math.max(0, at - 2)
  const count = Math.max(FIRST, at - from + 1)
  const tail = from + count >= ids.length
  return { tail, first: tail ? null : (ids[from] ?? null), count }
}

export function rangeOf(ids: readonly string[], span: Span): { from: number; to: number } {
  const at = span.tail || span.first === null ? -1 : ids.indexOf(span.first)
  if (at < 0) return { from: Math.max(0, ids.length - span.count), to: ids.length }
  return { from: at, to: Math.min(ids.length, at + span.count) }
}

/* `ids` are the messages in order; `guess` is a row's height before it is drawn. */
export function useWindow(ids: readonly string[], guess: (id: string) => number, box: React.RefObject<HTMLElement | null>): Windowed {
  const sizes = useRef(new Map<string, number>())
  /* Guessed once: a spacer carries a row's height while it is out of view, and it must not move there. */
  const guesses = useRef(new Map<string, number>())
  const guessing = useRef(guess)
  guessing.current = guess
  const [span, setSpan] = useState<Span>({ tail: true, first: null, count: FIRST })
  const latest = useRef({ ids, span })
  latest.current = { ids, span }
  /* A row on screen and where it sat, kept across a move that draws rows above it. */
  const anchor = useRef<{ id: string; at: number } | null>(null)
  /* A message asked to be shown, waiting for its page or its draw. */
  const wanted = useRef<{ id: string; then?: (row: HTMLElement) => void; moved: number } | null>(null)
  const [, nudge] = useState(0)
  const reveal = useCallback((id: string, then?: (row: HTMLElement) => void) => {
    wanted.current = { id, then, moved: 0 }
    nudge((n) => n + 1)
  }, [])

  const height = useCallback((id: string) => {
    const measured = sizes.current.get(id)
    if (measured !== undefined) return measured
    let guessed = guesses.current.get(id)
    if (guessed === undefined) guesses.current.set(id, (guessed = guessing.current(id)))
    return guessed
  }, [])
  const { from, to } = rangeOf(ids, span)
  const kept = useRef<Spacers | null>(null)
  kept.current = spacers(kept.current, ids, from, to, (at) => height(ids[at]!))
  const { before, after } = kept.current
  const measure = useCallback((id: string, row: HTMLElement) => {
    if (row.offsetHeight > 0) sizes.current.set(id, row.offsetHeight + GAP)
  }, [])

  const look = useCallback(() => {
    const scroll = box.current
    /* Not laid out (a hidden tab): nothing to measure the view against. */
    if (!scroll || scroll.clientHeight === 0) return
    const { ids: list, span: now } = latest.current
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
    setSpan({ tail, first: tail ? null : (list[next.from] ?? null), count: next.to - next.from })
  }, [box, height])

  /* Measured after every draw, and the row read before a move put back where it was. */
  useLayoutEffect(() => {
    const scroll = box.current
    if (!scroll) return
    for (const row of Array.from(scroll.querySelectorAll<HTMLElement>('.trow'))) if (row.dataset.id) measure(row.dataset.id, row)
    const want = wanted.current
    if (want) {
      const row = Array.from(scroll.querySelectorAll<HTMLElement>('.trow')).find((one) => one.dataset.id === want.id)
      const at = latest.current.ids.indexOf(want.id)
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
        setSpan(spanAround(latest.current.ids, at))
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
  useEffect(look, [ids.length, look])

  return { from, to, before, after, reveal, measure }
}
