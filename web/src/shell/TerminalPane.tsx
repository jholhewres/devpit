import { FitAddon } from '@xterm/addon-fit'
import { Terminal } from '@xterm/xterm'
import '@xterm/xterm/css/xterm.css'
import { useEffect, useRef, useState } from 'react'

import { attach, scrollback, type Attached } from './attach'
import { ask, commands } from './live'
import { useShell } from './useShell'

/* One client id per window: the backend hands a pane to one attacher at a
   time, and a reload has to be able to take it back from the last one. */
const CLIENT = crypto.randomUUID()

export function TerminalPane(): React.JSX.Element {
  const { project } = useShell()
  const host = useRef<HTMLDivElement>(null)
  const [error, setError] = useState<string | null>(null)

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
    fit.fit()

    let live: Attached | null = null
    let dropped = false

    void (async () => {
      const ensured = await ask(() => commands.sessionEnsure(project.id, null))
      if (ensured.error) return setError(ensured.error)
      const layout = await ask(() => commands.sessionLayout(project.id))
      const leaf = layout.data?.focusedId
      if (!leaf) return setError('no pane in this session')

      /* Scrollback first, then the stream: the other order shows new output
         above what came before it. */
      const past = await scrollback(project.id, leaf)
      if (past) term.write(past)

      const attached = await attach(project.id, leaf, CLIENT, { rows: term.rows, cols: term.cols }, (bytes) =>
        term.write(bytes),
      )
      if (dropped) return attached?.detach()
      live = attached
      term.onData((data) => live?.write(data))
    })()

    const watch = new ResizeObserver(() => {
      fit.fit()
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
  }, [project])

  return (
    <>
      {error && <div className="exempty__t">{error}</div>}
      <div className="termhost" ref={host} />
    </>
  )
}
