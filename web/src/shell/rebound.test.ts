import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

type Keys = Record<string, readonly string[]>

const backend = vi.hoisted(() => ({
  stored: {} as Keys,
  refuse: false,
  read: null as Promise<void> | null,
  writes: [] as Keys[],
}))

vi.mock('./live', () => ({
  ask: async (call: () => Promise<unknown>) => {
    try {
      return { data: await call(), error: null, loading: false }
    } catch (thrown) {
      return { data: null, error: String(thrown), loading: false }
    }
  },
  commands: {
    /* What was stored when asked, however late the answer arrives. */
    reboundRead: async () => {
      const now = { ...backend.stored }
      await backend.read
      return { keys: now }
    },
    reboundSet: async ({ keys }: { keys: Keys }) => {
      if (backend.refuse) throw new Error('the store is busy')
      backend.writes.push(keys)
      backend.stored = { ...keys }
      return { keys }
    },
  },
}))

const LEGACY = 'devpit:shortcuts'

/* A window starting: the module reads this window's storage as it loads. */
const start = async () => {
  vi.resetModules()
  return import('./rebound')
}

beforeEach(() => {
  backend.stored = {}
  backend.refuse = false
  backend.read = null
  backend.writes = []
  localStorage.clear()
})
afterEach(() => localStorage.clear())

describe('the keys a person recorded', () => {
  it('answer at once and are written to the settings', async () => {
    const { keepRebound, rebound } = await start()
    keepRebound({ palette: ['Mod+J'] })
    expect(rebound()).toEqual({ palette: ['Mod+J'] })
    await vi.waitFor(() => expect(backend.writes).toEqual([{ palette: ['Mod+J'] }]))
    expect(localStorage.getItem(LEGACY)).toBeNull()
  })

  it('come back from the settings when the window starts', async () => {
    backend.stored = { palette: ['Mod+J'] }
    const { loadRebound, rebound } = await start()
    await loadRebound()
    expect(rebound()).toEqual({ palette: ['Mod+J'] })
    expect(backend.writes).toEqual([])
  })

  it('move from the window’s own storage to the settings, once', async () => {
    localStorage.setItem(LEGACY, JSON.stringify({ palette: ['Mod+J'] }))
    const first = await start()
    /* Not the defaults while the settings are being asked. */
    expect(first.rebound()).toEqual({ palette: ['Mod+J'] })
    await first.loadRebound()
    expect(backend.writes).toEqual([{ palette: ['Mod+J'] }])
    expect(localStorage.getItem(LEGACY)).toBeNull()

    const second = await start()
    await second.loadRebound()
    expect(second.rebound()).toEqual({ palette: ['Mod+J'] })
    expect(backend.writes).toHaveLength(1)
  })

  it('stay in the window’s storage when the move could not be written', async () => {
    localStorage.setItem(LEGACY, JSON.stringify({ palette: ['Mod+J'] }))
    backend.refuse = true
    const { loadRebound, rebound } = await start()
    await loadRebound()
    expect(rebound()).toEqual({ palette: ['Mod+J'] })
    expect(localStorage.getItem(LEGACY)).not.toBeNull()
  })

  it('are the settings’ when both have some, and the window’s copy goes', async () => {
    backend.stored = { palette: ['Mod+J'] }
    localStorage.setItem(LEGACY, JSON.stringify({ palette: ['Mod+L'] }))
    const { loadRebound, rebound } = await start()
    await loadRebound()
    expect(rebound()).toEqual({ palette: ['Mod+J'] })
    expect(backend.writes).toEqual([])
    expect(localStorage.getItem(LEGACY)).toBeNull()
  })

  it('are not undone by a read that answers after a key was recorded', async () => {
    backend.stored = { palette: ['Mod+L'] }
    let answer = (): void => {}
    backend.read = new Promise((resolve) => (answer = resolve))
    const { keepRebound, loadRebound, rebound } = await start()
    const loading = loadRebound()
    keepRebound({ palette: ['Mod+J'] })
    answer()
    await loading
    expect(rebound()).toEqual({ palette: ['Mod+J'] })
  })
})
