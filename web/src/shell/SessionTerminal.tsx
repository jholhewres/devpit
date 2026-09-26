import { useEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'

import type { LiveSession } from '../gen/bindings'
import { Leaf, PANE_FREED } from './Leaf'
import { ask, commands } from './live'

/*
 * A session's own terminal, opened over the orchestrator's chat: the same
 * pane its tab shows, live, and typed into as the person — so it answers what
 * the session asks, which a message from the orchestrator never can.
 *
 * A pane is drawn by one view at a time. Opening it here takes it from its
 * tab; closing this gives it back, and the tab attaches again.
 *
 * Sized by its edges and corners and always centred, so a drag grows it on
 * both sides at once; the size is kept for the next one opened.
 */

const KEPT = 'devpit.sessionTerminal.size'
const SMALLEST = { width: 560, height: 320 }
/* Room left around it, so the window behind stays in view. */
const MARGIN = 32

interface Size {
  width: number
  height: number
}

const fits = (size: Size): Size => ({
  width: Math.round(Math.min(Math.max(size.width, SMALLEST.width), window.innerWidth - MARGIN)),
  height: Math.round(Math.min(Math.max(size.height, SMALLEST.height), window.innerHeight - MARGIN)),
})

function keptSize(): Size {
  try {
    const kept = JSON.parse(localStorage.getItem(KEPT) ?? 'null') as Size | null
    if (kept && kept.width > 0 && kept.height > 0) return fits(kept)
  } catch {
    // Unread, it opens at its default.
  }
  return fits({ width: Math.min(1100, window.innerWidth * 0.86), height: Math.min(760, window.innerHeight * 0.82) })
}

/* Which way an edge grows it: -1 for the left or top, 1 for the right or
   bottom, 0 for an axis the handle does not move. */
type Edge = { x: -1 | 0 | 1; y: -1 | 0 | 1; name: string }
const EDGES: readonly Edge[] = [
  { x: 1, y: 0, name: 'e' },
  { x: -1, y: 0, name: 'w' },
  { x: 0, y: 1, name: 's' },
  { x: 0, y: -1, name: 'n' },
  { x: 1, y: 1, name: 'se' },
  { x: -1, y: 1, name: 'sw' },
  { x: 1, y: -1, name: 'ne' },
  { x: -1, y: -1, name: 'nw' },
]

export function SessionTerminal({ session, onGo, onClose }: { session: LiveSession; onGo: () => void; onClose: () => void }): React.JSX.Element | null {
  const pane = session.pane
  const [size, setSize] = useState<Size>(keptSize)
  const [dragging, setDragging] = useState(false)
  /* Closing the terminal itself — not just this view of it — asks once. */
  const [ending, setEnding] = useState(false)
  const [refused, setRefused] = useState<string | null>(null)
  const end = (): void => {
    if (!pane) return
    void ask(() => commands.terminalClose(pane.projectId, pane.paneId)).then((done) => {
      if (done.error) return setRefused(done.error)
      onClose()
    })
  }
  const latest = useRef(size)
  latest.current = size

  useEffect(() => {
    if (!pane) return
    return () => {
      window.dispatchEvent(new CustomEvent(PANE_FREED, { detail: pane.paneId }))
    }
  }, [pane])

  /* A window made smaller keeps it inside. */
  useEffect(() => {
    const kept = (): void => setSize((was) => fits(was))
    window.addEventListener('resize', kept)
    return () => window.removeEventListener('resize', kept)
  }, [])

  const grab = (edge: Edge) => (event: React.PointerEvent<HTMLSpanElement>): void => {
    event.preventDefault()
    const from = { x: event.clientX, y: event.clientY, ...latest.current }
    const handle = event.currentTarget
    handle.setPointerCapture(event.pointerId)
    setDragging(true)
    /* Centred, an edge moved by d grows it by 2d. */
    const move = (moved: PointerEvent): void =>
      setSize(
        fits({
          width: from.width + edge.x * 2 * (moved.clientX - from.x),
          height: from.height + edge.y * 2 * (moved.clientY - from.y),
        }),
      )
    const done = (): void => {
      handle.removeEventListener('pointermove', move)
      handle.removeEventListener('pointerup', done)
      handle.removeEventListener('pointercancel', done)
      setDragging(false)
      try {
        localStorage.setItem(KEPT, JSON.stringify(latest.current))
      } catch {
        // Unsaved, it is still this size now.
      }
    }
    handle.addEventListener('pointermove', move)
    handle.addEventListener('pointerup', done)
    handle.addEventListener('pointercancel', done)
  }

  if (!pane) return null

  return createPortal(
    <div className="ask sterm__scrim" data-open="true" onClick={(event) => event.target === event.currentTarget && onClose()}>
      <div className="sterm" role="dialog" aria-modal="true" aria-label={`${session.name}'s terminal`} data-dragging={dragging ? 'true' : undefined} style={{ width: size.width, height: size.height }}>
        <header className="sterm__bar">
          <i className="deleg__dot" data-state={session.waiting ? 'waiting' : session.status === 'busy' ? 'busy' : 'idle'} />
          <span className="sterm__name">{session.name}</span>
          <span className="sterm__where">{[session.projectName, session.cardId ? 'card' : null].filter(Boolean).join(' · ')}</span>
          <span className="sterm__size" aria-hidden="true">
            {dragging ? `${size.width} × ${size.height}` : ''}
          </span>
          {ending ? (
            <>
              <span className="sterm__warn">{refused ?? 'Close this terminal and whatever runs in it?'}</span>
              <button className="sterm__btn" onClick={() => (setEnding(false), setRefused(null))}>
                Keep it
              </button>
              <button className="sterm__btn sterm__btn--bad" onClick={end}>
                Close terminal
              </button>
            </>
          ) : (
            <button className="sterm__btn" onClick={() => setEnding(true)} title="Close the terminal itself, and whatever runs in it">
              Close terminal…
            </button>
          )}
          <button className="sterm__btn" onClick={onGo} title="Open its project and tab">
            Go there
            <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round"><path d="M7 17 17 7M8 7h9v9" /></svg>
          </button>
          <button className="sterm__x" onClick={onClose} aria-label="Hide the terminal" title="Hide — it keeps running">
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round"><path d="M18 6 6 18M6 6l12 12" /></svg>
          </button>
        </header>
        <div className="sterm__body">
          <Leaf paneId={pane.paneId} projectId={pane.projectId} cwd={session.cwd} />
        </div>
        {EDGES.map((edge) => (
          <span key={edge.name} className="sterm__grip" data-edge={edge.name} onPointerDown={grab(edge)} aria-hidden="true" />
        ))}
      </div>
    </div>,
    document.querySelector('.app') ?? document.body,
  )
}
