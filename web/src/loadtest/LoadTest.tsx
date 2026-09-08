import { useCallback, useRef, useState } from 'react'
import { Channel, invoke } from '@tauri-apps/api/core'
import { Terminal } from '@xterm/xterm'
import '@xterm/xterm/css/xterm.css'

/**
 * Throughput gate for the terminal path.
 *
 * Pushes a large flood out of a real pty, over the binary channel, into
 * xterm.js, and reports what was measured — not what was hoped. A terminal
 * that stalls the window under a verbose build is a terminal nobody keeps
 * open, so the number is worth having before anything is built on top.
 *
 * `pty_drain` is deliberately absent from the generated contract: Tauri's one
 * binary response cannot be described by the type generator. Its wrapper is
 * written by hand here, and only here.
 */
type Throughput = {
  bytes: number
  frames: number
  millis: number
  bytesPerSecond: number
}

type Measured = Throughput & {
  /** Bytes the webview actually received, counted on this side. */
  receivedBytes: number
  /** Longest gap between animation frames during the flood, in ms. */
  worstFrameGapMs: number
  /** Frames rendered per second while the flood ran. */
  fpsDuringFlood: number
}

const MEGABYTE = 1024 * 1024

export function LoadTest(): React.JSX.Element {
  const host = useRef<HTMLDivElement>(null)
  const [running, setRunning] = useState(false)
  const [measured, setMeasured] = useState<Measured | null>(null)
  const [failure, setFailure] = useState<string | null>(null)

  const run = useCallback(async (megabytes: number) => {
    if (!host.current) return
    setRunning(true)
    setFailure(null)
    setMeasured(null)

    host.current.replaceChildren()
    const term = new Terminal({
      convertEol: true,
      scrollback: 1000,
      fontSize: 11,
      theme: { background: '#0c0c0e' }
    })
    term.open(host.current)

    // The responsiveness probe. A UI that freezes does not miss "some" frames
    // — it stops painting entirely, and the gap between rAF callbacks is the
    // measurement that shows it. Claiming "it stayed responsive" without this
    // number would be the same unfounded assertion the observatory exists to
    // reject.
    let received = 0
    let paints = 0
    let worstGap = 0
    let lastPaint = performance.now()
    let painting = true

    const paint = (now: number): void => {
      const gap = now - lastPaint
      if (gap > worstGap) worstGap = gap
      lastPaint = now
      paints += 1
      if (painting) requestAnimationFrame(paint)
    }
    requestAnimationFrame(paint)

    const onFrame = new Channel<ArrayBuffer | number[]>()
    onFrame.onmessage = (frame) => {
      // Raw frames arrive as an ArrayBuffer. Anything else means the binary
      // path silently fell back to JSON, which is worth knowing loudly.
      const bytes =
        frame instanceof ArrayBuffer ? new Uint8Array(frame) : new Uint8Array(frame)
      received += bytes.byteLength
      term.write(bytes)
    }

    const started = performance.now()
    try {
      const report = await invoke<Throughput>('pty_drain', {
        command: `yes quockpit | head -c ${megabytes * MEGABYTE}`,
        onFrame
      })

      const elapsed = performance.now() - started
      painting = false

      setMeasured({
        ...report,
        receivedBytes: received,
        worstFrameGapMs: Math.round(worstGap),
        fpsDuringFlood: Math.round((paints / elapsed) * 1000)
      })
    } catch (thrown) {
      painting = false
      setFailure(thrown instanceof Error ? thrown.message : String(thrown))
    } finally {
      setRunning(false)
    }
  }, [])

  return (
    <section className="loadtest">
      <header>
        <h2>PTY load test</h2>
        <div className="actions">
          <button type="button" disabled={running} onClick={() => void run(10)}>
            10 MB
          </button>
          <button type="button" disabled={running} onClick={() => void run(50)}>
            50 MB
          </button>
        </div>
      </header>

      {failure ? <p className="failure">{failure}</p> : null}

      {measured ? (
        <dl className="measured">
          <dt>sent</dt>
          <dd>{(measured.bytes / MEGABYTE).toFixed(1)} MB</dd>
          <dt>received</dt>
          <dd>{(measured.receivedBytes / MEGABYTE).toFixed(1)} MB</dd>
          <dt>throughput</dt>
          <dd>{(measured.bytesPerSecond / MEGABYTE).toFixed(1)} MB/s</dd>
          <dt>frames</dt>
          <dd>{measured.frames}</dd>
          <dt>elapsed</dt>
          <dd>{measured.millis} ms</dd>
          <dt>fps during flood</dt>
          <dd>{measured.fpsDuringFlood}</dd>
          <dt>worst frame gap</dt>
          <dd data-bad={measured.worstFrameGapMs > 250}>{measured.worstFrameGapMs} ms</dd>
        </dl>
      ) : null}

      <div className="terminal" ref={host} />
    </section>
  )
}
