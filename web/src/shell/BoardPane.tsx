import { Fragment, useEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'


import type { Card } from '../gen/bindings'
import { LaneFoot, LaneHead, Tile } from './Lane'
import { useBoard } from './useBoard'
import { useShell } from './useShell'

/*
 * The board, and the one gesture that is the whole point of it.
 *
 * Pointer events rather than HTML5 drag-and-drop: the native API cannot style
 * its own drag image, and half the point here is that the card in the air
 * looks lifted.
 *
 * The original stays where it was, dimmed, and a slot opens where the card
 * will land — so the answer to "where does this go" is on screen while there
 * is still time to change it.
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

export function BoardPane(): React.JSX.Element {
  const { project } = useShell()
  const live = useBoard(project?.id ?? null)
  const lanes = live.lanes
  const [held, setHeld] = useState<Held | null>(null)
  const [landing, setLanding] = useState<Landing | null>(null)
  const [landed, setLanded] = useState<string | null>(null)
  const surface = useRef<HTMLDivElement>(null)

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

  function onDown(event: React.PointerEvent, card: Card, from: string): void {
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

  function onMove(event: React.PointerEvent): void {
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

  function onUp(): void {
    if (!held) return
    const { card, from, moved } = held
    setHeld(null)
    if (!moved || !landing) {
      setLanding(null)
      return
    }
    live.move(card.id, landing.lane, landing.index)
    setLanding(null)
    if (landing.lane !== from) {
      setLanded(card.id)
      window.setTimeout(() => setLanded(null), 640)
    }
  }

  return (
    <div className="board" ref={surface} onPointerMove={onMove} onPointerUp={onUp} onPointerCancel={onUp}>
      {lanes.map((lane) => {
        const dropping = landing?.lane === lane.column.id
        return (
          <div
            key={lane.column.id}
            className="blane"
            data-lane={lane.column.id}
            data-agent={lane.column.step?.name}
            data-over={String(Boolean(held?.moved && dropping))}
          >
            <LaneHead lane={lane} onRename={(name) => live.renameColumn(lane.column.id, name)} />

            <div className="blane__list">
              {lane.cards.map((card, index) => (
                <Fragment key={card.id}>
                  {dropping && held?.moved && landing.index === index && (
                    <div className="slot" style={{ height: held.height }} />
                  )}
                  <div
                    className={landed === card.id ? 'tile tile--landed' : 'tile'}
                    role="button"
                    tabIndex={0}
                    data-ctx="card"
                    data-card={card.id}
                    data-ghost={String(held?.card.id === card.id && held.moved)}
                    onPointerDown={(event) => onDown(event, card, lane.column.id)}
                  >
                    <Tile card={card} />
                  </div>
                </Fragment>
              ))}
              {dropping && held?.moved && landing.index >= lane.cards.length && (
                <div className="slot" style={{ height: held.height }} />
              )}
            </div>

            <LaneFoot
              lane={lane}
              onAddCard={() => live.addCard(lane.column.id, 'New card')}
              onDelete={() => live.deleteColumn(lane.column.id)}
            />
          </div>
        )
      })}

      {/* On the body, not in the board.
          `position: fixed` inside an ancestor that has a transform anchors to
          that ancestor instead of the window, and the pane's entrance
          animation is exactly such an ancestor — which puts the card in the
          air nowhere near the pointer. */}
      <button className="blane__new" onClick={() => live.addColumn('New column')}>
        + Column
      </button>


      {held?.moved &&
        createPortal(
          <div
            className="tile tile--float"
            style={{ width: held.width, left: held.x - held.dx, top: held.y - held.dy }}
          >
            <Tile card={held.card} />
          </div>,
          document.body,
        )}
    </div>
  )
}
