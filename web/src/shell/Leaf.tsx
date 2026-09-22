import { FitAddon } from '@xterm/addon-fit'
import { Terminal } from '@xterm/xterm'
import '@xterm/xterm/css/xterm.css'
import { useEffect, useRef, useState } from 'react'

import { attach, scrollback, type Attached } from './attach'
import { drawOnTheGpu, measureWideCharacters } from './terminalAddons'
import { reason } from './reason'
import { ask, commands } from './live'
import { contrastFor, darkNow, options, palette } from './terminal'
import { useMarks } from './useMarks'

/*
 * One terminal, attached to one pane.
 *
 * A leaf of the tab's tree. It knows nothing about the tree: which pane it is
 * and which project it belongs to is all it takes, so a split rearranging its
 * siblings never tears it down and loses its attachment.
 */

export function Leaf({
  paneId,
  projectId,
}: {
  paneId: string
  projectId: string
}): React.JSX.Element {
  const host = useRef<HTMLDivElement>(null)
  const [error, setError] = useState<string | null>(null)
  const [term, setTerm] = useState<Terminal | null>(null)

  useEffect(() => {
    const box = host.current
    if (!projectId || !box) return

    const terminal = new Terminal(options(darkNow()))
    /* The person's contrast, when they chose one. Read after the terminal is
       up rather than before: a settings read must not hold the pane empty. */
    let contrast: number | null = null
    void ask(() => commands.settingsRead()).then((answer) => {
      contrast = answer.data?.terminalContrast ?? null
      terminal.options.minimumContrastRatio = contrastFor(darkNow(), contrast)
    })
    const fit = new FitAddon()
    terminal.loadAddon(fit)
    terminal.open(box)
    // After `open`, which is when there is a canvas to take a context from.
    measureWideCharacters(terminal)
    drawOnTheGpu(terminal)

    /* The pane is `display: none` until its tab is active and animates in on a
       transform, so a fit in this tick measures nothing and xterm ends up with
       zero columns. Fit only once the box has a size. */
    const refit = (): void => {
      if (box.clientWidth < 2 || box.clientHeight < 2) return
      fit.fit()
    }
    requestAnimationFrame(refit)

    let live: Attached | null = null
    let dropped = false

    void (async () => {
      /* Scrollback first, then the stream, or new output lands above what came
         before it. */
      const past = await scrollback(paneId)
      if (past) terminal.write(past)

      const attached = attach(
        projectId,
        paneId,
        { rows: terminal.rows, cols: terminal.cols },
        (bytes) => terminal.write(bytes),
        /* The pane ended. A failure here is the one the person needs to see:
           it is why the terminal is empty. */
        (failed) => {
          if (!dropped && failed) setError(failed)
        },
      )
      if (dropped) return attached?.detach()
      live = attached
      terminal.onData((data) => live?.write(data))
      setTerm(terminal)
    })().catch((thrown: unknown) => {
      /* Every step above talks to the backend, and a rejection here used to
         vanish: the pane mounted, drew a cursor, and never attached. */
      if (!dropped) setError(reason(thrown, 'the terminal could not be opened'))
    })

    /* The theme changes under a live terminal. Repainting the palette is
       enough — rebuilding the pane would drop the scrollback and the process
       along with it. */
    const themed = new MutationObserver(() => {
      terminal.options.theme = palette(darkNow())
      terminal.options.minimumContrastRatio = contrastFor(darkNow(), contrast)
    })
    themed.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] })

    /* The observer fires for every pixel of a drag, per visible pane. The pty
       only cares about the grid, so it is told only when rows or columns
       actually changed. */
    const watch = new ResizeObserver(() => {
      const before = { rows: terminal.rows, cols: terminal.cols }
      refit()
      if (terminal.rows === before.rows && terminal.cols === before.cols) return
      void live?.resize(terminal.rows, terminal.cols).then((applied) => {
        /* The pty clamps; the grid follows what it actually got. */
        if (applied && (applied.rows !== terminal.rows || applied.cols !== terminal.cols)) {
          terminal.resize(applied.cols, applied.rows)
        }
      })
    })
    watch.observe(box)

    /* A pane is `display: none` while another tab is in front. The rows
       survive that; the pixels do not, and coming back the grid is the same
       size it was — so the `ResizeObserver` above says nothing and the
       terminal stays blank until a click happens to force a render.
       Coming back into view is the force. */
    const shown =
      typeof IntersectionObserver === 'undefined'
        ? null
        : new IntersectionObserver((entries) => {
            if (!entries.some((entry) => entry.isIntersecting)) return
            refit()
            terminal.refresh(0, terminal.rows - 1)
          })
    shown?.observe(box)

    return () => {
      dropped = true
      setTerm(null)
      themed.disconnect()
      watch.disconnect()
      shown?.disconnect()
      live?.detach()
      terminal.dispose()
    }
  }, [projectId, paneId])

  useMarks(term, paneId)

  return (
    <>
      {error && <div className="exempty__t">{error}</div>}
      <div className="termhost" ref={host} />
    </>
  )
}
