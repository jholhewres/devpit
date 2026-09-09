import { EDGES, startResize, type Edge } from './window'

/*
 * The eight grips a frameless window is pulled by.
 *
 * They sit above everything and are invisible; only the cursor says they are
 * there, which is what a window edge does. The corners come last so they win
 * where they overlap an edge — a diagonal pull is what you meant if you
 * aimed at a corner.
 */
export function ResizeEdges(): React.JSX.Element {
  return (
    <>
      {EDGES.map((edge: Edge) => (
        <span
          key={edge}
          className="grip"
          data-edge={edge}
          aria-hidden="true"
          /* pointerdown, not click: the compositor takes the pointer for the
             rest of the gesture and no click ever lands. */
          onPointerDown={(event) => {
            if (event.button !== 0) return
            event.preventDefault()
            startResize(edge)
          }}
        />
      ))}
    </>
  )
}
