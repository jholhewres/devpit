import { afterEach, describe, expect, it, vi } from 'vitest'

/* A plain function, not `vi.fn`: a spy follows the promise it returns, and
   that alone counts as handling the rejection this test is about. */
const core = vi.hoisted(() => {
  const asked: string[] = []
  const gone = { code: 'not_found', message: 'that pane is not attached', retryAfterMs: null, details: null }
  return {
    asked,
    invoke: (command: string): Promise<unknown> => {
      asked.push(command)
      if (command === 'panes_socket') return Promise.resolve('ws://127.0.0.1:1/panes?k=x')
      return command === 'session_attach' ? new Promise(() => {}) : Promise.reject(gone)
    },
    Channel: class {
      onmessage: ((frame: unknown) => void) | null = null
    },
  }
})
vi.mock('@tauri-apps/api/core', () => core)

import { attach, attachPane, retryIn } from './attach'
import { LOST, UNAVAILABLE } from './paneSocket'

afterEach(() => {
  delete (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__
  core.asked.length = 0
})

describe('an attached pane', () => {
  /* The pty can end between two keystrokes; the next one is refused, and that
     refusal is no error worth a report. */
  it('lets a keystroke into a pane that has gone go without a loose rejection', async () => {
    ;(window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {}
    const pane = attach('p', 'leaf_1', { rows: 24, cols: 80 }, () => {})
    const loose = vi.fn()
    process.on('unhandledRejection', loose)
    try {
      pane?.write('x')
      pane?.detach()
      await new Promise((settled) => setTimeout(settled, 0))
      expect(core.asked).toEqual(['session_attach', 'session_write', 'session_detach'])
      expect(loose).not.toHaveBeenCalled()
    } finally {
      process.off('unhandledRejection', loose)
    }
  })

  /* A policy or a server that refuses the socket leaves the pane on the
     channel, not on a blank screen retrying forever. */
  it('sends the next attach over the channel once the socket could not be opened', async () => {
    ;(window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {}
    vi.stubGlobal('WebSocket', function () {
      throw new Error('SecurityError: The operation is insecure.')
    })
    try {
      const ended: (string | null)[] = []
      const on = { output: () => {}, end: (why: string | null) => ended.push(why) }
      await attachPane('p', 'leaf_1', { rows: 24, cols: 80 }, on)
      await new Promise((settled) => setTimeout(settled, 0))
      expect(ended).toEqual([UNAVAILABLE])
      expect(core.asked).not.toContain('session_attach')
      await attachPane('p', 'leaf_1', { rows: 24, cols: 80 }, on)
      expect(core.asked).toContain('session_attach')
    } finally {
      vi.unstubAllGlobals()
    }
  })
})

describe('attaching again after an attach ended', () => {
  const browser = (yes: boolean): void => {
    ;(window as unknown as { __DEVPIT_BROWSER__?: boolean }).__DEVPIT_BROWSER__ = yes
  }
  afterEach(() => browser(false))

  it('falls back to the channel at once in the window', () => {
    expect(retryIn(UNAVAILABLE, 0)).toBe(0)
    expect(retryIn(LOST, 3)).toBe(1000)
  })

  /* A browser has no channel: the socket is tried again, never in a loop
     that spins, and never so slowly the terminal seems dead. */
  it('waits longer each time in a browser, up to a ceiling', () => {
    browser(true)
    const waits = [0, 1, 2, 3, 4, 5, 6].map((tries) => retryIn(UNAVAILABLE, tries)!)
    expect(waits[0]).toBeGreaterThan(0)
    for (let i = 1; i < waits.length; i += 1) expect(waits[i]).toBeGreaterThanOrEqual(waits[i - 1]!)
    expect(waits.at(-1)).toBe(waits.at(-2))
  })

  it('does not try again for a pane that ended or failed for good', () => {
    expect(retryIn(null, 0)).toBeNull()
    expect(retryIn('the pane is gone', 0)).toBeNull()
  })

  it('keeps the socket in a browser, so the next attach tries it again', async () => {
    browser(true)
    let opened = 0
    vi.stubGlobal('WebSocket', function () {
      opened += 1
      throw new Error('refused')
    })
    try {
      /* Fresh: the window's tests above left the channel chosen. */
      vi.resetModules()
      const fresh = await import('./attach')
      const on = { output: () => {}, end: () => {} }
      await fresh.attachPane('p', 'leaf_1', { rows: 24, cols: 80 }, on)
      await new Promise((settled) => setTimeout(settled, 0))
      await fresh.attachPane('p', 'leaf_1', { rows: 24, cols: 80 }, on)
      await new Promise((settled) => setTimeout(settled, 0))
      expect(opened).toBe(2)
    } finally {
      vi.unstubAllGlobals()
    }
  })
})
