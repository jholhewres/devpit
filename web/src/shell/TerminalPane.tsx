import { FitAddon } from '@xterm/addon-fit'
import { Terminal } from '@xterm/xterm'
import '@xterm/xterm/css/xterm.css'
import { useEffect, useRef, useState } from 'react'

import { attach, scrollback, type Attached } from './attach'
import { ask, commands } from './live'
import type { Tab } from './strip'
import { useShell } from './useShell'

/* One client id per window: the backend hands a pane to one attacher at a
   time, and a reload has to be able to take it back from the last one. */
const CLIENT = crypto.randomUUID()

export function TerminalPane({ tab }: { tab: Tab }): React.JSX.Element {
  const { project, open, rename, attach: remember } = useShell()
  const host = useRef<HTMLDivElement>(null)
  const [error, setError] = useState<string | null>(null)
  const first = open.find((other) => other.kind === 'term')?.id === tab.id

  useEffect(() => {
    const box = host.current
    if (!project || !box) return

    const term = new Terminal({
      fontFamily: 'Geist Mono, ui-monospace, monospace',
      fontSize: 12,
      lineHeight: 1.35,
      cursorBlink: true,
      allowProposedApi: true,
      theme: { background: '#151515', foreground: '#e2e2e2' },
    })
    const fit = new FitAddon()
    term.loadAddon(fit)
    term.open(box)

    /* The pane is `display: none` until its tab is active and animates in on
       a transform, so a fit in this tick measures nothing and xterm ends up
       with zero columns. Fit only once the box has a size. */
    const refit = (): void => {
      if (box.clientWidth < 2 || box.clientHeight < 2) return
      fit.fit()
    }
    requestAnimationFrame(refit)

    /* The shell reports where it is; the tab says what the shell said. */
    term.onTitleChange((title) => rename(tab.id, title))

    let live: Attached | null = null
    let dropped = false

    void (async () => {
      let leaf = tab.paneId
      if (!leaf) {
        const ensured = await ask(() => commands.sessionEnsure(project.id, null))
        if (ensured.error) return setError(ensured.error)
        const layout = await ask(() => commands.sessionLayout(project.id))
        if (!layout.data) return setError(layout.error ?? 'no session')
        /* The first terminal takes the pane the session already has; every
           one after it splits a new leaf of its own. */
        leaf = first
          ? layout.data.focusedId
          : (await ask(() =>
              commands.sessionSplit(project.id, layout.data!.focusedId, 'vertical', null),
            ).then((split) => split.data?.focusedId))
        if (!leaf) return setError('could not open a pane')
        remember(tab.id, leaf)
      }

      /* Scrollback first, then the stream, or new output lands above what
         came before it. */
      const past = await scrollback(project.id, leaf)
      if (past) term.write(past)

      const attached = await attach(
        project.id,
        leaf,
        CLIENT,
        { rows: term.rows, cols: term.cols },
        (bytes) => term.write(bytes),
      )
      if (dropped) return attached?.detach()
      live = attached
      term.onData((data) => live?.write(data))
    })()

    const watch = new ResizeObserver(() => {
      refit()
      void live?.resize(term.rows, term.cols).then((applied) => {
        /* The pty clamps; the grid follows what it actually got. */
        if (applied && (applied.rows !== term.rows || applied.cols !== term.cols)) {
          term.resize(applied.cols, applied.rows)
        }
      })
    })
    watch.observe(box)

    return () => {
      dropped = true
      watch.disconnect()
      live?.detach()
      term.dispose()
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [project, tab.id])

  return (
    <>
      {error && <div className="exempty__t">{error}</div>}
      <div className="termhost" ref={host} />
    </>
  )
}
