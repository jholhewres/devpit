import { FitAddon } from '@xterm/addon-fit'
import { Terminal } from '@xterm/xterm'
import '@xterm/xterm/css/xterm.css'
import { useCallback, useEffect, useRef, useState } from 'react'

import { attach, scrollback, type Attached } from './attach'
import { drawOnTheGpu, measureWideCharacters } from './terminalAddons'
import { reason } from './reason'
import { ask, commands } from './live'
import { contrastFor, darkNow, options, palette } from './terminal'
import { clipboardKey, copySelection, pasteClipboard } from './terminalClipboard'
import { TerminalMenu } from './TerminalMenu'
import { guardComposition } from './terminalIme'
import { picturesAsPaths } from './terminalPaste'
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
  onSplit,
  onClosePane,
}: {
  paneId: string
  projectId: string
  /** What the right-click menu offers for the pane, from the tab that owns it. */
  onSplit?: (direction: 'horizontal' | 'vertical') => void
  onClosePane?: () => void
}): React.JSX.Element {
  const host = useRef<HTMLDivElement>(null)
  const [error, setError] = useState<string | null>(null)
  /* Beside the terminal, not instead of it: a paste that failed leaves the
     terminal as usable as it was. */
  const [pasteFailed, setPasteFailed] = useState<string | null>(null)
  const [term, setTerm] = useState<Terminal | null>(null)
  const [menuAt, setMenuAt] = useState<{ x: number; y: number } | null>(null)

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
    const pasted = picturesAsPaths(projectId, (path) => terminal.paste(path), setPasteFailed)
    box.addEventListener('paste', pasted, true)
    /* Copy and paste keys, taken before xterm's textarea sees them: on
       WebKitGTK neither reaches a clipboard from there (`terminalClipboard`). */
    const keys = (event: KeyboardEvent): void => {
      const act = clipboardKey(event)
      if (!act) return
      event.preventDefault()
      event.stopPropagation()
      if (act === 'copy') void copySelection(terminal)
      else void pasteClipboard(terminal, projectId).then(setPasteFailed)
    }
    box.addEventListener('keydown', keys, true)
    /* The terminal's own right-click, kept from the window's menu, which
       leaves text fields — and xterm's input is one — to the webview. */
    const menu = (event: MouseEvent): void => {
      event.preventDefault()
      event.stopPropagation()
      setMenuAt({ x: event.clientX, y: event.clientY })
    }
    box.addEventListener('contextmenu', menu, true)
    const unguard = guardComposition(box, (data) => terminal.input(data, true))

    /* The pane is `display: none` until its tab is active and animates in on a
       transform, so a fit in this tick measures nothing and xterm ends up with
       zero columns. Fit only once the box has a size. */
    let live: Attached | null = null
    let dropped = false
    /* What the pty was last told, not what the grid was a moment ago: a fit
       that ran while nothing was attached yet — or on coming back into view,
       where the box did not change size — moved the grid and told nobody, and
       every later comparison against the grid found nothing to send. The pty
       stayed at 80×24 under a terminal twice as wide. */
    let told: { rows: number; cols: number } | null = null
    /* A resize sent while the attach is still being claimed is refused; it is
       asked again shortly, a few times, rather than lost. */
    let retries = 0
    /* The grid follows the box at once; the pty hears about it once the box
       has stopped moving. A split animating open is a dozen sizes in a few
       frames, and each one is a SIGWINCH the agent answers by redrawing —
       Claude Code clears its screen to do that, and a burst of them left it
       blank or drawn at a size already gone. */
    let settling: ReturnType<typeof setTimeout> | undefined
    const refit = (): void => {
      if (box.clientWidth >= 2 && box.clientHeight >= 2) fit.fit()
      clearTimeout(settling)
      settling = setTimeout(tell, 60)
    }
    const tell = (): void => {
      const { rows, cols } = terminal
      if (dropped || !live || (told?.rows === rows && told.cols === cols)) return
      told = { rows, cols }
      void live.resize(rows, cols).then((applied) => {
        /* The pty clamps; the grid follows what it actually got. */
        if (applied && (applied.rows !== terminal.rows || applied.cols !== terminal.cols)) {
          told = { rows: applied.rows, cols: applied.cols }
          terminal.resize(applied.cols, applied.rows)
        }
        retries = 0
      }, () => {
        told = null
        if (!dropped && retries++ < 10) setTimeout(refit, 200)
      })
    }
    requestAnimationFrame(refit)

    void (async () => {
      /* Scrollback first, then the stream, or new output lands above what came
         before it. */
      const past = await scrollback(paneId)
      /* A mount dropped while that was on its way must not attach at all: an
         attach arriving after the live mount's is the newer claim, wins the
         pane, and is let go at once — leaving the terminal on screen with
         nobody attached and "a newer client already owns that pane". React
         mounts twice in development, so there it was every restart. */
      if (dropped) return
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
      told = { rows: terminal.rows, cols: terminal.cols }
      /* The grid may have been fitted while the attach was on its way. */
      refit()
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
    const watch = new ResizeObserver(refit)
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
      clearTimeout(settling)
      setTerm(null)
      themed.disconnect()
      watch.disconnect()
      shown?.disconnect()
      box.removeEventListener('paste', pasted, true)
      box.removeEventListener('keydown', keys, true)
      box.removeEventListener('contextmenu', menu, true)
      unguard()
      live?.detach()
      terminal.dispose()
    }
  }, [projectId, paneId])

  useMarks(term, paneId)
  const closeMenu = useCallback(() => setMenuAt(null), [])

  return (
    <>
      {error && <div className="exempty__t">{error}</div>}
      {pasteFailed && <div className="exempty__t">{pasteFailed}</div>}
      <div className="termhost" ref={host} />
      {menuAt && term && (
        <TerminalMenu at={menuAt} terminal={term} projectId={projectId} onClose={closeMenu} onFailed={setPasteFailed} onSplit={onSplit} onClosePane={onClosePane} />
      )}
    </>
  )
}
