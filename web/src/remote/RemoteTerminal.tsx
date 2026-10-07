import { FitAddon } from '@xterm/addon-fit'
import { Terminal } from '@xterm/xterm'
import { useEffect, useRef, useState } from 'react'

import type { RemoteIn } from '../gen/bindings'
import { SESSION_COMMANDS } from '../shell/sessionCommands'
import { base64Of, bytesOf } from './socket'

/*
 * One terminal of the machine, watched. Its size here is its own: the
 * machine attaches this viewer so it changes nothing on the desk. Typing is
 * keys straight through, a key bar for what a phone keyboard lacks, and a
 * line written and sent whole.
 */

const KEYS: readonly { label: string; bytes: string }[] = [
  { label: 'Esc', bytes: '\x1b' },
  { label: 'Tab', bytes: '\t' },
  { label: '^C', bytes: '\x03' },
  { label: '↑', bytes: '\x1b[A' },
  { label: '↓', bytes: '\x1b[B' },
  { label: '←', bytes: '\x1b[D' },
  { label: '→', bytes: '\x1b[C' },
  { label: 'Enter', bytes: '\r' },
]

export function RemoteTerminal({
  project,
  pane,
  typing,
  send,
  bytes,
}: {
  project: string
  pane: string
  typing: boolean
  send: (message: RemoteIn) => void
  bytes: React.RefObject<(pane: string, b64: string) => void>
}): React.JSX.Element {
  const host = useRef<HTMLDivElement>(null)
  const [line, setLine] = useState('')

  useEffect(() => {
    if (!host.current) return
    const term = new Terminal({ fontSize: 12, scrollback: 2000, convertEol: false, disableStdin: !typing, cursorBlink: typing })
    const fit = new FitAddon()
    term.loadAddon(fit)
    term.open(host.current)
    fit.fit()
    bytes.current = (from, b64) => from === pane && term.write(bytesOf(b64))
    send({ t: 'paneOpen', project, pane, cols: term.cols, rows: term.rows })
    const typed = typing ? term.onData((data) => send({ t: 'paneInput', pane, b64: base64Of(data) })) : null
    return () => {
      typed?.dispose()
      send({ t: 'paneClose', pane })
      bytes.current = () => {}
      term.dispose()
    }
  }, [project, pane, typing, send, bytes])

  const sendLine = (): void => {
    if (!line.trim()) return
    send({ t: 'panePaste', pane, text: line })
    setLine('')
  }

  return (
    <div className="rm__term">
      <div className="rm__xterm" ref={host} />
      {typing ? (
        <>
          <div className="rm__keys">
            {KEYS.map((key) => (
              <button key={key.label} className="rm__key" onClick={() => send({ t: 'paneInput', pane, b64: base64Of(key.bytes) })}>
                {key.label}
              </button>
            ))}
          </div>
          <div className="rm__keys" aria-label="Commands">
            {SESSION_COMMANDS.map((one) => (
              <button key={one.command} className="rm__key" title={one.what} onClick={() => confirm(`Type ${one.command} in this terminal?`) && send({ t: 'panePaste', pane, text: one.command })}>
                {one.command}
              </button>
            ))}
          </div>
          <div className="rm__line">
            <input value={line} placeholder="Write, and send it as a line" onChange={(event) => setLine(event.target.value)} onKeyDown={(event) => event.key === 'Enter' && !event.nativeEvent.isComposing && sendLine()} />
            <button className="rm__btn" onClick={sendLine}>
              Send
            </button>
          </div>
        </>
      ) : (
        <p className="rm__dim">Watching only. Typing is allowed on the machine, per device.</p>
      )}
    </div>
  )
}
