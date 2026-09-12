import type { Terminal } from '@xterm/xterm'
import { useEffect, useRef } from 'react'

import type { Happening } from '../gen/bindings'
import { jump, marked, noMarks, toneOf, type Marks, type Said } from './marks'
import { onHappening } from './window'

/*
 * The commands a pane has run, marked in its margin.
 *
 * A terminal is a wall of bytes until the shell says where one command ends
 * and the next begins. It does now — `crates/pty/src/shell.rs` makes it — and
 * this is what that buys: a mark beside every prompt, green or red once the
 * command settles, and a way to jump between them without reading.
 *
 * The marks are drawn from the same `terminal:happening` an attached pane
 * emits, so a pane that was being listened to while you were elsewhere has its
 * history the moment you look at it.
 */

/** How far back a pane keeps marks. Past this the oldest are dropped: the
 *  terminal's own scrollback has a floor too, and a marker below it is a
 *  decoration on a row that no longer exists. */
const MOST = 512

export function useMarks(term: Terminal | null, paneId: string | null): void {
  const marks = useRef<Marks>(noMarks)
  /* Disposed together: a decoration outlives its marker otherwise, and a
     terminal that is torn down with live decorations leaks the elements. */
  const drawn = useRef<Array<{ dispose: () => void }>>([])

  useEffect(() => {
    if (!term || !paneId) return
    marks.current = noMarks

    const off = onHappening((happening: Happening) => {
      if (happening.paneId !== paneId) return
      const said = happening.what as Said
      const before = marks.current
      const row = term.buffer.active.baseY + term.buffer.active.cursorY
      marks.current = marked(before, said, happening.detail, row)
      if (marks.current === before) return
      if (marks.current.blocks.length > MOST) {
        marks.current = { blocks: marks.current.blocks.slice(-MOST) }
      }
      draw(term, marks.current, drawn, said)
    })

    /* Alt+Up and Alt+Down, which is what every terminal that has this uses.
       Returning false lets the key through to the shell, so a program that
       wants Alt+Up still gets it when there is nowhere to jump. */
    const disposeKeys = term.attachCustomKeyEventHandler((event) => {
      if (event.type !== 'keydown' || !event.altKey || event.ctrlKey || event.metaKey) return true
      if (event.key !== 'ArrowUp' && event.key !== 'ArrowDown') return true
      const here = term.buffer.active.viewportY
      const to = jump(marks.current, here, event.key === 'ArrowUp' ? 'back' : 'forward')
      if (to === null) return true
      term.scrollToLine(to)
      return false
    })

    return () => {
      off()
      /* `attachCustomKeyEventHandler` returns void in this version; the
         handler dies with the terminal. Kept as a name so the intent is
         readable if it ever returns a disposable. */
      void disposeKeys
      for (const one of drawn.current) one.dispose()
      drawn.current = []
    }
  }, [term, paneId])
}

/* One decoration per settled command. Drawn only when a command ends: a mark
   that appeared at the prompt and changed colour twice would be three
   repaints of a row nobody is looking at yet. */
function draw(
  term: Terminal,
  marks: Marks,
  drawn: React.MutableRefObject<Array<{ dispose: () => void }>>,
  said: Said,
): void {
  if (said !== 'finished') return
  const block = marks.blocks[marks.blocks.length - 1]
  if (!block) return
  const tone = toneOf(block)
  if (tone === 'none') return

  const marker = term.registerMarker(block.at - (term.buffer.active.baseY + term.buffer.active.cursorY))
  if (!marker) return
  const decoration = term.registerDecoration({ marker, x: 0, width: 1, layer: 'top' })
  if (!decoration) {
    marker.dispose()
    return
  }
  decoration.onRender((element) => {
    element.classList.add('cmdmark', `cmdmark--${tone}`)
  })
  drawn.current.push({
    dispose: () => {
      decoration.dispose()
      marker.dispose()
    },
  })
  if (drawn.current.length > MOST) drawn.current.splice(0, drawn.current.length - MOST)[0]?.dispose()
}
