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

import { attach, attachPane } from './attach'
import { UNAVAILABLE } from './paneSocket'

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
