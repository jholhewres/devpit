import { useEffect, useState } from 'react'

import type { Card } from '../gen/bindings'

/*
 * A card in the air.
 *
 * Pointer events rather than HTML5 drag-and-drop: the native API cannot style
 * its own drag image, and half the point of this board is that the card being
 * moved looks lifted.
 *
 * Apart from the board because it is a gesture and the board is a layout —
 * and because the gesture has six pieces of state of its own, which was most
 * of what `BoardPane` had grown to be.
 */

interface Held {
  readonly card: Card
  readonly from: string
  readonly width: number
  readonly height: number
  readonly dx: number
  readonly dy: number
  readonly ox: number
  readonly oy: number
  x: number
  y: number
  moved: boolean
}

interface Landing {
  readonly lane: string
  readonly index: number
}


export interface Drag {
  readonly held: Held | null
  readonly landing: Landing | null
  readonly landed: string | null
  down: (event: React.PointerEvent, card: Card, from: string) => void
  move: (event: React.PointerEvent) => void
  /** Ends it. Answers what happened, because a press that never moved is a
      click and the board is what knows what a click means. */
  up: () => Ended
}

/** A press that never became a drag is a click; anything else is a move. */
export type Ended =
  | { readonly what: 'clicked'; readonly card: Card }
  | { readonly what: 'moved'; readonly card: Card; readonly lane: string; readonly index: number }
  | { readonly what: 'nothing' }

export function useDrag(): Drag {
  const [held, setHeld] = useState<Held | null>(null)
  const [landing, setLanding] = useState<Landing | null>(null)
  const [landed, setLanded] = useState<string | null>(null)

  /* The cursor belongs to the whole window while a card is in the air — and
     the cleanup matters: unmounting mid-drag would leave every cursor in the
     app stuck on `grabbing` with nothing to release it. */
  useEffect(() => {
    if (held) document.body.dataset.dragging = 'true'
    else delete document.body.dataset.dragging
    return () => {
      delete document.body.dataset.dragging
    }
  }, [held])

  const down = (event: React.PointerEvent, card: Card, from: string): void => {
    if (event.button !== 0) return
    const box = (event.currentTarget as HTMLElement).getBoundingClientRect()
    event.currentTarget.setPointerCapture(event.pointerId)
    event.preventDefault()
    setHeld({
      card,
      from,
      width: box.width,
      height: box.height,
      dx: event.clientX - box.left,
      dy: event.clientY - box.top,
      ox: event.clientX,
      oy: event.clientY,
      x: event.clientX,
      y: event.clientY,
      moved: false,
    })
    setLanding(null)
  }

  const move = (event: React.PointerEvent): void => {
    if (!held) return
    /* A few pixels of slop, so a press that wanders while you decide still
       counts as a click rather than a drag to the same place. */
    const moved =
      held.moved ||
      Math.abs(event.clientX - held.ox) > 4 ||
      Math.abs(event.clientY - held.oy) > 4
    setHeld({ ...held, x: event.clientX, y: event.clientY, moved })
    if (!moved) return

    /* The float has `pointer-events: none`, so it never hides what is under
       the pointer from this. */
    const under = document.elementFromPoint(event.clientX, event.clientY)
    const lane = (under as HTMLElement | null)?.closest('.blane') as HTMLElement | null
    if (!lane?.dataset.lane) {
      setLanding(null)
      return
    }
    /* Which card the pointer is past, by midpoint — the rule every board
       uses, and the reason a slot never flickers between two rows. */
    const tiles = Array.from(lane.querySelectorAll<HTMLElement>('.blane__list .tile')).filter(
      (tile) => tile.dataset.card !== held.card.id,
    )
    const at = tiles.findIndex((tile) => {
      const box = tile.getBoundingClientRect()
      return event.clientY < box.top + box.height / 2
    })
    setLanding({ lane: lane.dataset.lane, index: at === -1 ? tiles.length : at })
  }

  const up = (): Ended => {
    if (!held) return { what: 'nothing' }
    const { card, from, moved } = held
    setHeld(null)
    setLanding(null)
    if (!moved) return { what: 'clicked', card }
    if (!landing) return { what: 'nothing' }

    /* The flash only for a card that changed lane: one reordered where it
       already was did not go anywhere worth pointing at. */
    if (landing.lane !== from) {
      setLanded(card.id)
      window.setTimeout(() => setLanded(null), 640)
    }
    return { what: 'moved', card, lane: landing.lane, index: landing.index }
  }

  return { held, landing, landed, down, move, up }
}
