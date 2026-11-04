import { FitAddon } from '@xterm/addon-fit'
import { Terminal } from '@xterm/xterm'
import '@xterm/xterm/css/xterm.css'
import { useCallback, useEffect, useRef, useState } from 'react'

import { attachPane, type Pane } from './attach'
import { LOST, UNAVAILABLE } from './paneSocket'
import { writeBatcher } from './writeBatcher'
import { gpuWhileShown, measureWideCharacters } from './terminalAddons'
import { reason } from './reason'
import { ask, commands } from './live'
import { contrastFor, darkNow, options, palette } from './terminal'
import { clipboardKey, copySelection, pasteClipboard } from './terminalClipboard'
import { TerminalMenu } from './TerminalMenu'
import { guardComposition } from './terminalIme'
import { picturesAsPaths } from './terminalPaste'
import { wheelFor } from './terminalWheel'
import { useMarks } from './useMarks'
import { menuPoint } from './menuRules'
import { useShellPick } from './shellStore'
import { linkPaths } from './terminalPathLinks'
import { shellWord, useFileDrop } from './fileDrop'

/*
 * One terminal, attached to one pane.
 *
 * A leaf of the tab's tree. It knows nothing about the tree: which pane it is
 * and which project it belongs to is all it takes, so a split rearranging its
 * siblings never tears it down and loses its attachment.
 */

/** Said when a view that took a pane from its tab lets it go. */
export const PANE_FREED = 'devpit:pane-freed'

/* Off screen this long, a pane gives back its xterm, its GPU context and its
   stream; the pty keeps running. Shorter, a switch back pays a rebuild. */
export const HIDDEN_DISPOSE_MS = 20_000

/* What an attach wrote, kept to rebuild the terminal around an older page.
   Past this, paging ends rather than holding a flood in memory. */
const HELD_MOST = 2 * 1024 * 1024

export function Leaf({
  paneId,
  projectId,
  onSplit,
  onClosePane,
  onTerminal,
  onBlocks,
  onSeparate,
  cwd = null,
  target,
}: {
  paneId: string
  projectId: string
  /** What the right-click menu offers for the pane, from the tab that owns it. */
  onSplit?: (direction: 'horizontal' | 'vertical') => void
  onClosePane?: () => void
  /** The live terminal, to whoever draws around it (`BlockTerm`). */
  onTerminal?: (terminal: Terminal | null) => void
  /** Offered on the right-click when the pane is drawn plain: back to blocks. */
  onBlocks?: () => void
  onSeparate?: () => void
  /** The shell's folder, when it says so: where a relative path it prints is. */
  cwd?: string | null
  /** The pane's target in the layout, which says whether tmux draws it. */
  target?: string
}): React.JSX.Element {
  const host = useRef<HTMLDivElement>(null)
  const [error, setError] = useState<string | null>(null)
  /* Beside the terminal, not instead of it: a paste that failed leaves the
     terminal as usable as it was. */
  const [pasteFailed, setPasteFailed] = useState<string | null>(null)
  const [term, setTerm] = useState<Terminal | null>(null)
  const [menuAt, setMenuAt] = useState<{ x: number; y: number } | null>(null)
  /* Another view held this pane a while — a session opened beside an
     orchestrator's chat — and gave it back: attached again, from scratch. */
  const [round, setRound] = useState(0)
  useEffect(() => {
    const back = (event: Event): void => {
      if ((event as CustomEvent<string>).detail === paneId) setRound((was) => was + 1)
    }
    window.addEventListener(PANE_FREED, back)
    return () => window.removeEventListener(PANE_FREED, back)
  }, [paneId])
  /* Whether the pane is on screen, or was a moment ago: off it, nothing of
     the terminal is kept but this element. */
  const [onScreen, setOnScreen] = useState(true)
  useEffect(() => {
    const box = host.current
    if (!box || typeof IntersectionObserver === 'undefined') return
    let hiding: ReturnType<typeof setTimeout> | undefined
    const seen = new IntersectionObserver((entries) => {
      clearTimeout(hiding)
      if (entries.some((entry) => entry.isIntersecting)) return setOnScreen(true)
      hiding = setTimeout(() => setOnScreen(false), HIDDEN_DISPOSE_MS)
    })
    seen.observe(box)
    return () => {
      clearTimeout(hiding)
      seen.disconnect()
    }
  }, [])
  const onTerminalRef = useRef(onTerminal)
  onTerminalRef.current = onTerminal
  /* Where a path clicked in the terminal opens: read at the click, so the
     terminal is not rebuilt when the project or the tabs change. */
  const opening = useShellPick((shell) => ({ show: shell.show, root: shell.project?.rootPath ?? null }))
  const openingRef = useRef({ ...opening, cwd })
  openingRef.current = { ...opening, cwd }

  useEffect(() => {
    const box = host.current
    if (!projectId || !box || !onScreen) return

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
    const linked = linkPaths(terminal, openingRef)
    // After `open`, which is when there is a canvas to take a context from.
    measureWideCharacters(terminal)
    /* tmux sends only what changes, so a screen that was emptied under it —
       a rebuilt terminal, a lost GPU context, a pane coming back — stays blank
       but for the cursor until the program next prints. Asked for whole. */
    const redraw = (tries = 5): void =>
      void ask(() => commands.sessionRedraw(projectId, paneId)).then((drawn) => {
        /* A client still connecting is not listed yet: asked again shortly. */
        if (drawn.data === false && tries > 1 && !dropped) setTimeout(() => redraw(tries - 1), 200)
      })
    const gpu = gpuWhileShown(terminal, () => redraw())
    /* Where there is no observer to say the pane is on screen, it is assumed
       to be; otherwise the observer's first answer turns the GPU on. */
    if (typeof IntersectionObserver === 'undefined') gpu.show()
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
      setMenuAt(menuPoint(event))
    }
    box.addEventListener('contextmenu', menu, true)
    const unguard = guardComposition(box, (data) => terminal.input(data, true))
    const wheel = wheelFor(target, terminal, box, projectId, paneId)

    /* The pane is `display: none` until its tab is active and animates in on a
       transform, so a fit in this tick measures nothing and xterm ends up with
       zero columns. Fit only once the box has a size. */
    let live: Pane | null = null
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
    let releasing: ReturnType<typeof setTimeout> | undefined
    /* At most once a frame: the observer fires for every pixel of a drag or
       of the side panel sliding, and each fit measures the box. */
    let framed = false
    const refit = (): void => {
      if (framed) return
      framed = true
      requestAnimationFrame(() => {
        framed = false
        if (dropped) return
        if (box.clientWidth >= 2 && box.clientHeight >= 2) fit.fit()
        clearTimeout(settling)
        wheel.dispose()
        settling = setTimeout(tell, 60)
      })
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

    /* Live output at most once a frame; parsed bytes acknowledged, which is
       what lets the pty read on. */
    const batch = writeBatcher(terminal, (bytes) => live?.ack(bytes))
    let held: Uint8Array[] | null = []
    let heldBytes = 0
    const hold = (bytes: Uint8Array): void => {
      if (!held) return
      heldBytes += bytes.length
      if (heldBytes > HELD_MOST) held = null
      else held.push(bytes)
    }
    let paging = false
    let more = true
    /* Scrolled to the top of a plain screen: the page before, and the
       terminal written again around it. xterm cannot prepend, so it is reset
       in order — after whatever is still waiting — and rewritten whole. */
    const page = async (): Promise<void> => {
      if (!live || !held || paging || !more || terminal.buffer.active.type !== 'normal') return
      paging = true
      const got = await live.older()
      paging = false
      if (dropped || !held) return
      if (!got || got.bytes.length === 0) {
        more = false
        return
      }
      more = got.left > 0
      const lines = terminal.buffer.active.length
      held.unshift(got.bytes)
      heldBytes += got.bytes.length
      batch.flush()
      /* The reset scrolls to the top too: no page is asked for until the
         rewrite is in. */
      paging = true
      terminal.write('\x1bc')
      for (const one of held) terminal.write(one)
      terminal.write(new Uint8Array(), () => {
        terminal.scrollToLine(Math.max(0, terminal.buffer.active.length - lines))
        paging = false
      })
      if (heldBytes > HELD_MOST) held = null
    }
    const scrolled = terminal.onScroll((top) => {
      if (top === 0) void page()
    })
    let drawn = false
    const draw = (bytes: Uint8Array, replay: boolean): void => {
      hold(bytes)
      /* The replay comes before any live byte and is not acknowledged. */
      if (replay) terminal.write(bytes)
      else batch.push(bytes)
      if (!drawn) {
        drawn = true
        box.dataset.drawn = 'true'
      }
    }

    void (async () => {
      const attached = await attachPane(
        projectId,
        paneId,
        { rows: terminal.rows, cols: terminal.cols },
        {
          output: (bytes, replay) => {
            if (!dropped) draw(bytes, replay)
          },
          /* The pane ended. A failure here is the one the person needs to see:
             it is why the terminal is empty. A connection that closed under
             it is attached again in a moment. */
          end: (failed) => {
            if (dropped) return
            if (failed === LOST || failed === UNAVAILABLE)
              setTimeout(() => !dropped && setRound((was) => was + 1), failed === LOST ? 1000 : 0)
            else if (failed) setError(failed)
          },
        },
      )
      /* A mount dropped while that was on its way must not keep the pane: an
         attach arriving after the live mount's is the newer claim, wins the
         pane, and is let go at once — leaving the terminal on screen with
         nobody attached and "a newer client already owns that pane". React
         mounts twice in development, so there it was every restart. */
      if (dropped) return attached?.detach()
      live = attached
      told = { rows: terminal.rows, cols: terminal.cols }
      /* The grid may have been fitted while the attach was on its way. */
      refit()
      setTimeout(() => !dropped && redraw(), 150)
      terminal.onData((data) => wheel.typed(data, (typed) => live?.write(typed)))
      setTerm(terminal)
      onTerminalRef.current?.(terminal)
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
            /* Given back only after a while off screen: a quick look at another
               tab and back should not rebuild the renderer and its glyphs. */
            if (!entries.some((entry) => entry.isIntersecting)) {
              clearTimeout(releasing)
              releasing = setTimeout(() => gpu.hide(), 20_000)
              return
            }
            clearTimeout(releasing)
            gpu.show()
            refit()
            terminal.refresh(0, terminal.rows - 1)
            if (live) redraw(1)
          })
    shown?.observe(box)

    return () => {
      dropped = true
      clearTimeout(settling)
      clearTimeout(releasing)
      setTerm(null)
      onTerminalRef.current?.(null)
      themed.disconnect()
      watch.disconnect()
      shown?.disconnect()
      box.removeEventListener('paste', pasted, true)
      box.removeEventListener('keydown', keys, true)
      box.removeEventListener('contextmenu', menu, true)
      unguard()
      linked.dispose()
      live?.detach()
      batch.dispose()
      scrolled.dispose()
      delete box.dataset.drawn
      terminal.dispose()
    }
  }, [projectId, paneId, round, onScreen, target])

  useMarks(term, paneId)
  const closeMenu = useCallback(() => setMenuAt(null), [])
  /* Files dropped on a terminal are typed at its prompt as their paths —
     quoted, and left for the person to run. */
  const dropped = useCallback((paths: readonly string[]) => term?.paste(`${paths.map(shellWord).join(' ')} `), [term])
  const hovering = useFileDrop(host, term ? dropped : null)

  return (
    <>
      {error && <div className="exempty__t">{error}</div>}
      {pasteFailed && <div className="exempty__t">{pasteFailed}</div>}
      <div className="termhost" ref={host} data-drop={hovering ? 'true' : undefined} />
      {menuAt && term && (
        <TerminalMenu at={menuAt} terminal={term} projectId={projectId} paneId={paneId} onClose={closeMenu} onFailed={setPasteFailed} onSplit={onSplit} onClosePane={onClosePane} onBlocks={onBlocks} onSeparate={onSeparate} />
      )}
    </>
  )
}
