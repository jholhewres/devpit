import { useEffect, useRef } from 'react'
import { Terminal } from '@xterm/xterm'
import { FitAddon } from '@xterm/addon-fit'
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

    const term = new Terminal({
      convertEol: false,
      cursorBlink: true,
      fontSize: 13,
      fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Consolas, monospace',
      theme: {
        background: '#0c0c0e',
        foreground: '#cdcdd4',
        cursor: '#cdcdd4'
      }
    })
    const fit = new FitAddon()
    term.loadAddon(fit)
    term.open(el)
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
