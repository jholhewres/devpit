import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import type { PaneName } from '../../shell/paneList'
import { opened, type Strip, type Tab } from '../../shell/strip'
import { DrawingList } from './DrawingList'
import { DRAWING } from './drawings'

afterEach(cleanup)

const files = new Map<string, { text: string; modified: number }>()
const written = vi.fn()
let refusal: { code: string; message: string } | null = null

vi.mock('../../shell/live', () => ({
  ask: async (call: () => unknown) => {
    try {
      return { data: await call(), error: null, loading: false }
    } catch (thrown) {
      const refused = thrown as { code: string; message: string }
      return { data: null, error: refused.message, code: refused.code, loading: false }
    }
  },
  commands: {
    pluginDataList: () => ({
      files: [...files].map(([name, file]) => ({ name, bytes: file.text.length, modified: file.modified })),
    }),
    pluginDataWrite: (_project: string, _plugin: string, name: string, text: string, expected: number | null) => {
      written(name, text, expected)
      if (refusal) throw refusal
      if ((files.get(name)?.modified ?? null) !== expected) throw { code: 'conflict', message: 'changed on disk' }
      files.set(name, { text, modified: Date.now() })
      return { name, modified: files.get(name)!.modified }
    },
  },
}))

/* `show` is the strip's own rule, so what is asserted is what the tab strip would do. */
let strip: Strip = { open: [], active: null }
const shell = {
  project: { id: 'p', name: 'demo' },
  close: vi.fn(),
  show: (kind: PaneName, tab: Partial<Tab> = {}) => {
    strip = opened(strip, { id: crypto.randomUUID(), kind, ...tab })
  },
}
vi.mock('../../shell/useShell', () => ({ useShell: () => shell }))

beforeEach(() => {
  files.clear()
  files.set('fluxo.excalidraw', { text: DRAWING.empty, modified: Date.now() })
  written.mockClear()
  refusal = null
  strip = { open: [], active: null }
})

const create = (typed: string): void => {
  fireEvent.change(screen.getByLabelText('New drawing name'), { target: { value: typed } })
  fireEvent.click(screen.getByRole('button', { name: 'Create' }))
}

describe('the list of drawings', () => {
  it('lists the drawings in the project by name, without the extension', async () => {
    render(<DrawingList tab={{ id: 'drawings', kind: 'drawing' }} />)
    expect(await screen.findByText('fluxo')).toBeTruthy()
  })

  it('creates a drawing from a typed name, and opens it', async () => {
    render(<DrawingList tab={{ id: 'drawings', kind: 'drawing' }} />)
    await screen.findByText('fluxo')
    create('mapa')
    await waitFor(() => expect(strip.active).toBe('drawing:mapa.excalidraw'))
    expect(written).toHaveBeenCalledWith('mapa.excalidraw', DRAWING.empty, null)
    expect(await screen.findByText('mapa')).toBeTruthy()
  })

  it('says why a name cannot be used before asking the backend', async () => {
    render(<DrawingList tab={{ id: 'drawings', kind: 'drawing' }} />)
    await screen.findByText('fluxo')
    create('notes/mapa')
    expect((await screen.findByRole('alert')).textContent).toMatch(/cannot/)
    expect(written).not.toHaveBeenCalled()
  })

  it("shows the backend's refusal when it has one", async () => {
    refusal = { code: 'invalid', message: '"mapa.excalidraw" is larger than Excalidraw allows' }
    render(<DrawingList tab={{ id: 'drawings', kind: 'drawing' }} />)
    await screen.findByText('fluxo')
    create('mapa')
    expect((await screen.findByRole('alert')).textContent).toBe(refusal.message)
    expect(strip.open).toEqual([])
  })

  it('says a name is taken rather than writing over that drawing', async () => {
    files.set('mapa.excalidraw', { text: 'theirs', modified: 1 })
    render(<DrawingList tab={{ id: 'drawings', kind: 'drawing' }} />)
    await screen.findByText('mapa')
    create('mapa')
    expect((await screen.findByRole('alert')).textContent).toBe('mapa already exists.')
    expect(files.get('mapa.excalidraw')!.text).toBe('theirs')
  })

  it('opens each drawing in a tab of its own', async () => {
    files.set('mapa.excalidraw', { text: DRAWING.empty, modified: Date.now() })
    render(<DrawingList tab={{ id: 'drawings', kind: 'drawing' }} />)
    fireEvent.click(await screen.findByText('fluxo'))
    fireEvent.click(screen.getByText('mapa'))
    expect(strip.open.map((tab) => tab.title)).toEqual(['fluxo', 'mapa'])
    expect(strip.active).toBe('drawing:mapa.excalidraw')
  })

  it('opening a drawing that is already open focuses its tab instead of opening another', async () => {
    strip = opened(strip, { id: 'term_1', kind: 'term', title: 'Terminal 1' })
    render(<DrawingList tab={{ id: 'drawings', kind: 'drawing' }} />)
    fireEvent.click(await screen.findByText('fluxo'))
    expect(strip.active).toBe('drawing:fluxo.excalidraw')

    strip = { ...strip, active: 'term_1' }
    fireEvent.click(screen.getByText('fluxo'))
    expect(strip.open.map((tab) => tab.id)).toEqual(['term_1', 'drawing:fluxo.excalidraw'])
    expect(strip.active).toBe('drawing:fluxo.excalidraw')
  })
})
