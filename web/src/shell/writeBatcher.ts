/*
 * What a pane prints, handed to xterm at most once a frame.
 *
 * A keystroke's echo (a small chunk with nothing waiting) goes straight in, so
 * typing never waits for a frame. Anything else is kept and written as one
 * `write` on the next frame: a flood is a few dozen parses a second, not one
 * per chunk. A hidden window runs no frames, so a timer flushes in its place,
 * or the pty would stall behind acks that never come.
 */

/** The largest chunk written at once when nothing is waiting: an echo. */
export const DIRECT_MOST = 512
/** Parsed bytes are acknowledged in groups this big. */
export const ACK_EVERY = 64 * 1024
/** The flush that stands in for a frame when none comes. */
const NO_FRAME_MS = 100

export interface Writable {
  write: (data: Uint8Array, done?: () => void) => void
}

export interface Clock {
  /** Runs `run` on the next frame; answers how to cancel it. */
  frame: (run: () => void) => () => void
}

const browserClock: Clock = {
  frame: (run) => {
    let ran = false
    const once = (): void => {
      if (ran) return
      ran = true
      cancelAnimationFrame(id)
      clearTimeout(timer)
      run()
    }
    const id = requestAnimationFrame(once)
    const timer = setTimeout(once, NO_FRAME_MS)
    return () => {
      ran = true
      cancelAnimationFrame(id)
      clearTimeout(timer)
    }
  },
}

export interface Batcher {
  push: (bytes: Uint8Array) => void
  /** Hands the terminal what waits now, ahead of anything written after. */
  flush: () => void
  dispose: () => void
}

export function writeBatcher(term: Writable, ack: (bytes: number) => void, clock: Clock = browserClock): Batcher {
  let queue: Uint8Array[] = []
  let cancel: (() => void) | null = null
  let parsed = 0
  let gone = false

  const write = (data: Uint8Array): void =>
    term.write(data, () => {
      if (gone) return
      parsed += data.length
      if (parsed >= ACK_EVERY) {
        ack(parsed)
        parsed = 0
      }
    })

  const flush = (): void => {
    cancel = null
    if (gone || queue.length === 0) return
    const waiting = queue
    queue = []
    if (waiting.length === 1) return write(waiting[0])
    const all = new Uint8Array(waiting.reduce((sum, one) => sum + one.length, 0))
    let at = 0
    for (const one of waiting) {
      all.set(one, at)
      at += one.length
    }
    write(all)
  }

  return {
    push: (bytes) => {
      if (gone || bytes.length === 0) return
      if (bytes.length <= DIRECT_MOST && queue.length === 0 && !cancel) return write(bytes)
      queue.push(bytes)
      cancel ??= clock.frame(flush)
    },
    flush: () => {
      cancel?.()
      flush()
    },
    dispose: () => {
      gone = true
      queue = []
      cancel?.()
      cancel = null
    },
  }
}
