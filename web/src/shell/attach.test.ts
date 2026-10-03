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
      return command === 'session_attach' ? new Promise(() => {}) : Promise.reject(gone)
    },
    Channel: class {
      onmessage: ((frame: unknown) => void) | null = null
    },
  }
})
vi.mock('@tauri-apps/api/core', () => core)

import { attach } from './attach'

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
})
