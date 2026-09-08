import { useEffect, useState } from 'react'
import { getCurrentWindow } from '@tauri-apps/api/window'

/**
 * The window frame, drawn by the app.
 *
 * `decorations: false` in `tauri.conf.json` takes the system titlebar away, so
 * everything it used to do has to exist here: dragging, the resize edges, and
 * the three controls. The reason is a row of pixels — a native titlebar sits
 * above the app in the system's own colours and steals 30-something pixels
 * from a window that is open all day, and no dark theme reaches it.
 *
 * What it costs is stated rather than discovered: the edges below are the
 * resize grip the window manager was providing for free.
 */

/**
 * The eight grab regions, in pixels.
 *
 * `EDGE` is what a window manager gives you and what a hand can actually hit —
 * four pixels is a target you miss, and missing it means clicking whatever row
 * happens to end at the window edge instead. `CORNER` is larger again because
 * a diagonal grab is aimed at a point, not a line.
 */
const EDGE = 7
const CORNER = 18

const EDGES = [
  { dir: 'North', style: { top: 0, left: CORNER, right: CORNER, height: EDGE, cursor: 'n-resize' } },
  { dir: 'South', style: { bottom: 0, left: CORNER, right: CORNER, height: EDGE, cursor: 's-resize' } },
  { dir: 'West', style: { left: 0, top: CORNER, bottom: CORNER, width: EDGE, cursor: 'w-resize' } },
  { dir: 'East', style: { right: 0, top: CORNER, bottom: CORNER, width: EDGE, cursor: 'e-resize' } },
  { dir: 'NorthWest', style: { top: 0, left: 0, width: CORNER, height: CORNER, cursor: 'nw-resize' } },
  { dir: 'NorthEast', style: { top: 0, right: 0, width: CORNER, height: CORNER, cursor: 'ne-resize' } },
  { dir: 'SouthWest', style: { bottom: 0, left: 0, width: CORNER, height: CORNER, cursor: 'sw-resize' } },
  { dir: 'SouthEast', style: { bottom: 0, right: 0, width: CORNER, height: CORNER, cursor: 'se-resize' } }
] as const

/**
 * The eight grab edges an undecorated window has to draw for itself.
 *
 * Rendered outside the shell rather than inside it: the shell clips its
 * overflow, and an edge that lives under a clipping ancestor is an edge you
 * cannot reach at the exact pixel where it matters.
 */
export function ResizeEdges(): React.JSX.Element {
  const start = (direction: string) => (event: React.PointerEvent) => {
    event.preventDefault()
    void getCurrentWindow()
      .startResizeDragging(direction as never)
      .catch(() => undefined)
  }

  return (
    <>
      {/* A visible boundary, because a frameless dark window on a dark desktop
          has none — and you cannot aim at an edge you cannot see. */}
      <div className="window-edge" aria-hidden="true" />
      {EDGES.map((edge) => (
        <div
          key={edge.dir}
          className="resize-edge"
          style={edge.style as React.CSSProperties}
          onPointerDown={start(edge.dir)}
        />
      ))}
    </>
  )
}

/**
 * Minimise, maximise, close.
 *
 * Drawn rather than reused from the system, because the system's are the ones
 * that came with the titlebar we removed. Ordered as the platform orders them
 * and sized to the 36px strip they sit in.
 */
export function WindowControls(): React.JSX.Element {
  const [maximized, setMaximized] = useState(false)
  const win = getCurrentWindow()

  // The window can be maximised by a double-click on the strip or by the
  // window manager, so the icon follows the window rather than our clicks.
  useEffect(() => {
    let alive = true
    const read = (): void => {
      void win.isMaximized().then((is) => {
        if (alive) setMaximized(is)
      })
    }
    read()
    const stop = win.onResized(read)
    return () => {
      alive = false
      // Caught, not assumed: a teardown that throws takes the unmount with it,
      // and losing the whole shell over an unsubscribe is a bad trade.
      void stop.then((off) => off()).catch(() => undefined)
    }
  }, [win])

  return (
    <div className="window-controls">
      <button
        type="button"
        className="window-control"
        title="Minimise"
        onClick={() => void win.minimize()}
      >
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
          <path d="M1 5h8" stroke="currentColor" strokeWidth="1.1" />
        </svg>
      </button>

      <button
        type="button"
        className="window-control"
        title={maximized ? 'Restore' : 'Maximise'}
        onClick={() => void win.toggleMaximize()}
      >
        <svg width="10" height="10" viewBox="0 0 10 10" fill="none" aria-hidden="true">
          {maximized ? (
            <>
              <rect x="1" y="2.6" width="6" height="6" stroke="currentColor" strokeWidth="1.1" />
              <path d="M3.2 2.4V1.2h5.6v5.6H7.4" stroke="currentColor" strokeWidth="1.1" />
            </>
          ) : (
            <rect x="1.2" y="1.2" width="7.6" height="7.6" stroke="currentColor" strokeWidth="1.1" />
          )}
        </svg>
      </button>

      <button
        type="button"
        className="window-control"
        data-close="true"
        title="Close"
        onClick={() => void win.close()}
      >
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true">
          <path d="M1.4 1.4l7.2 7.2M8.6 1.4L1.4 8.6" stroke="currentColor" strokeWidth="1.1" />
        </svg>
      </button>
    </div>
  )
}
