import { useEffect, useRef } from 'react'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
import { Unicode11Addon } from '@xterm/addon-unicode11'
import { WebLinksAddon } from '@xterm/addon-web-links'
import { WebglAddon } from '@xterm/addon-webgl'
import { terminalOptions } from './terminalOptions'
import { commands } from '../gen/bindings'
import { sessionAttach } from './attach'
import '@xterm/xterm/css/xterm.css'

/**
 * One leaf: an xterm talking to a tmux client pty.
 *
 * Mounted for the life of the leaf. A surface covers the stage; this stays
 * attached, which is what keeps scrollback when Overview opens.
 */
export function TerminalPane({
  projectId,
  paneId,
  focused,
  onFocus
}: {
  projectId: string
  paneId: string
  focused: boolean
  onFocus: () => void
}): React.JSX.Element {
  const host = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const el = host.current
    if (!el) return

    const term = new Terminal({ convertEol: false, ...terminalOptions() })
    const fit = new FitAddon()
    term.loadAddon(fit)

    // Wide characters and emoji measured by the Unicode 11 rules rather than
    // the 2018 defaults. Without it a prompt with an emoji in it puts every
    // character after it one cell off, for the rest of the line.
    const unicode = new Unicode11Addon()
    term.loadAddon(unicode)
    term.unicode.activeVersion = '11'

    term.loadAddon(new WebLinksAddon())
    term.open(el)

    // WebGL after `open`, and only if the context is there: a machine without
    // one — a VM, a remote session, a browser that lost the context — falls
    // back to the DOM renderer, which is slower and correct. Losing the
    // context later disposes the addon rather than leaving a dead canvas.
    let webgl: WebglAddon | null = null
    try {
      webgl = new WebglAddon()
      webgl.onContextLoss(() => {
        webgl?.dispose()
        webgl = null
      })
      term.loadAddon(webgl)
    } catch {
      webgl = null
    }

    fit.fit()

    const attach = sessionAttach(projectId, paneId, term.rows, term.cols, (bytes) => {
      term.write(bytes)
    })
    void attach.done.catch((thrown: unknown) => {
      const message = thrown instanceof Error ? thrown.message : String(thrown)
      term.writeln(`\r\n\x1b[31m${message}\x1b[0m`)
    })

    const write = term.onData((data) => {
      void commands.sessionWrite(paneId, data)
    })

    const sendSize = (): void => {
      fit.fit()
      void commands.sessionResize(paneId, term.rows, term.cols)
    }
    sendSize()

    const ro = new ResizeObserver(sendSize)
    ro.observe(el)

    return () => {
      attach.stop()
      ro.disconnect()
      write.dispose()
      webgl?.dispose()
      term.dispose()
    }
  }, [projectId, paneId])

  return (
    <div
      className="pty-leaf"
      data-focused={focused}
      data-testid={`pty-leaf-${paneId}`}
      onPointerDown={onFocus}
      ref={host}
    />
  )
}
