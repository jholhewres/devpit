import { act, cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'

import type { Tab } from '../../shell/strip'
import type { CanvasProps } from './Canvas'
import { Drawing } from './Drawing'
import { drawingTab, SAVE_AFTER_MS } from './drawings'

/* The package cannot load under jsdom. What is tested is what this plugin
   does with the canvas's callbacks, so the canvas is a stand-in that keeps
   the props it was last given. */
let canvas: CanvasProps | null = null
vi.mock('./Canvas', () => ({
  Canvas: (props: CanvasProps) => {
    canvas = props
    return <div data-testid="canvas" />
  },
}))

/* A data folder that behaves like the backend's: a write naming a modified
   time other than the one on disk is a conflict, and writes nothing. */
const files = new Map<string, { text: string; modified: number }>()
let clock = 100
const written = vi.fn()

/* The project's board, and what the backend was asked to pin. */
const CARDS = [
  { id: 'card_1', title: 'Wire the board' },
  { id: 'card_2', title: 'Ship it' },
]
let board = CARDS
let pinRefusal: { code: string; message: string } | null = null
const pinnedTo = vi.fn()

vi.mock('../../shell/live', () => ({
  ask: async (call: () => unknown) => {
    try {
      return { data: await call(), error: null, loading: false }
    } catch (thrown) {
      const refusal = thrown as { code: string; message: string }
      return { data: null, error: refusal.message, code: refusal.code, loading: false }
    }
  },
  commands: {
    pluginDataRead: (_project: string, _plugin: string, name: string) => {
      const file = files.get(name)
      if (!file) throw { code: 'not_found', message: `${name} is not there` }
      return { name, text: file.text, modified: file.modified }
    },
    pluginDataWrite: (_project: string, _plugin: string, name: string, text: string, expected: number | null) => {
      written(name, text, expected)
      if ((files.get(name)?.modified ?? null) !== expected) {
        throw { code: 'conflict', message: `${name} changed on disk since it was read` }
      }
      clock += 1
      files.set(name, { text, modified: clock })
      return { name, modified: clock }
    },
    pluginDataDelete: (_project: string, _plugin: string, name: string) => ({ removed: files.delete(name) }),
    boardGet: () => ({ cards: board }),
    pluginDataPin: (_project: string, plugin: string, name: string, cardId: string) => {
      pinnedTo(plugin, name, cardId)
      if (pinRefusal) throw pinRefusal
      return { pinned: [] }
    },
  },
}))

const shell = {
  project: { id: 'p', name: 'demo' },
  theme: 'system' as 'system' | 'light' | 'dark',
  show: vi.fn(),
  close: vi.fn(),
  markUnsaved: vi.fn(),
}
vi.mock('../../shell/useShell', () => ({ useShell: () => shell }))

let systemDark = false
beforeAll(() => {
  window.matchMedia = ((query: string) => ({
    matches: systemDark && query.includes('dark'),
    addEventListener: () => {},
    removeEventListener: () => {},
  })) as unknown as typeof window.matchMedia
})

const scene = (label: string): string => JSON.stringify({ type: 'excalidraw', version: 2, elements: [{ id: label }] })
const FIRST = scene('first')
const MINE = scene('mine')
const MORE = scene('more')
const THEIRS = scene('theirs')
const NAME = 'fluxo.excalidraw'
const tab: Tab = { id: `drawing:${NAME}`, kind: 'drawing', path: NAME, title: 'fluxo' }

beforeEach(() => {
  files.clear()
  files.set(NAME, { text: FIRST, modified: 5 })
  clock = 100
  written.mockClear()
  board = CARDS
  pinRefusal = null
  pinnedTo.mockClear()
  shell.show.mockClear()
  shell.close.mockClear()
  shell.theme = 'system'
  systemDark = false
  canvas = null
})

afterEach(() => {
  cleanup()
  vi.useRealTimers()
})

async function opened(): Promise<ReturnType<typeof render>> {
  const view = render(<Drawing tab={tab} name={NAME} />)
  await screen.findByTestId('canvas')
  return view
}

const draw = (text: string): void => act(() => canvas!.onChange(text))
const wait = (ms: number): Promise<void> => act(async () => void (await vi.advanceTimersByTimeAsync(ms)))

describe('a drawing saves itself', () => {
  it('writes 800 ms after the last change, with the modified time it read', async () => {
    await opened()
    vi.useFakeTimers()
    draw(MINE)
    await wait(500)
    draw(MORE)
    await wait(SAVE_AFTER_MS - 1)
    expect(written).not.toHaveBeenCalled()
    await wait(1)
    expect(written).toHaveBeenCalledTimes(1)
    expect(written).toHaveBeenCalledWith(NAME, MORE, 5)
    expect(files.get(NAME)!.text).toBe(MORE)
  })

  it('builds each save on the modified time the one before it returned', async () => {
    await opened()
    vi.useFakeTimers()
    draw(MINE)
    await wait(SAVE_AFTER_MS)
    draw(MORE)
    await wait(SAVE_AFTER_MS)
    expect(written).toHaveBeenLastCalledWith(NAME, MORE, 101)
    expect(screen.queryByRole('alert')).toBeNull()
  })

  it('writes nothing when the canvas reports the drawing it already has', async () => {
    await opened()
    vi.useFakeTimers()
    draw(FIRST)
    await wait(SAVE_AFTER_MS * 2)
    expect(written).not.toHaveBeenCalled()
  })

  it('writes what was drawn when the tab closes inside the wait', async () => {
    const view = await opened()
    vi.useFakeTimers()
    draw(MINE)
    view.unmount()
    await wait(0)
    expect(written).toHaveBeenCalledWith(NAME, MINE, 5)
  })
})

describe('a drawing changed on disk by someone else', () => {
  async function refused(): Promise<void> {
    await opened()
    files.set(NAME, { text: THEIRS, modified: 999 })
    vi.useFakeTimers()
    draw(MINE)
    await wait(SAVE_AFTER_MS)
  }

  it('offers Reload and Keep mine instead of writing over the change', async () => {
    await refused()
    expect(files.get(NAME)!.text).toBe(THEIRS)
    const notice = screen.getByRole('alert')
    expect(within(notice).getByRole('button', { name: 'Reload' })).toBeTruthy()
    expect(within(notice).getByRole('button', { name: 'Keep mine' })).toBeTruthy()
  })

  it('does not try again on its own while nobody has chosen', async () => {
    await refused()
    draw(MORE)
    await wait(SAVE_AFTER_MS * 3)
    expect(written).toHaveBeenCalledTimes(1)
    expect(files.get(NAME)!.text).toBe(THEIRS)
  })

  it('Keep mine reads the modified time again, then writes what is on screen', async () => {
    await refused()
    vi.useRealTimers()
    fireEvent.click(screen.getByRole('button', { name: 'Keep mine' }))
    await waitFor(() => expect(files.get(NAME)!.text).toBe(MINE))
    expect(written).toHaveBeenLastCalledWith(NAME, MINE, 999)
    expect(screen.queryByRole('alert')).toBeNull()
  })

  it('Reload starts the canvas again from what is on disk', async () => {
    await refused()
    vi.useRealTimers()
    fireEvent.click(screen.getByRole('button', { name: 'Reload' }))
    await waitFor(() => expect(canvas!.initialText).toBe(THEIRS))
    expect(screen.queryByRole('alert')).toBeNull()
    expect(files.get(NAME)!.text).toBe(THEIRS)

    vi.useFakeTimers()
    draw(MORE)
    await wait(SAVE_AFTER_MS)
    expect(written).toHaveBeenLastCalledWith(NAME, MORE, 999)
  })
})

describe("a drawing's name", () => {
  const renameTo = (typed: string): void => {
    fireEvent.click(screen.getByRole('button', { name: 'Rename drawing' }))
    const field = screen.getByLabelText('Name')
    fireEvent.change(field, { target: { value: typed } })
    fireEvent.keyDown(field, { key: 'Enter' })
  }

  it('changes by writing the new file, deleting the old one and moving the tab', async () => {
    await opened()
    renameTo('mapa')
    await waitFor(() => expect(shell.show).toHaveBeenCalledWith('drawing', drawingTab('mapa.excalidraw')))
    expect(shell.close).toHaveBeenCalledWith(tab.id)
    expect(written).toHaveBeenCalledWith('mapa.excalidraw', FIRST, null)
    expect(files.get('mapa.excalidraw')!.text).toBe(FIRST)
    expect(files.has(NAME)).toBe(false)
  })

  it('refuses a name another drawing has, and keeps this one', async () => {
    files.set('mapa.excalidraw', { text: THEIRS, modified: 7 })
    await opened()
    renameTo('mapa')
    expect(await screen.findByText('mapa already exists.')).toBeTruthy()
    expect(files.get('mapa.excalidraw')!.text).toBe(THEIRS)
    expect(files.get(NAME)!.text).toBe(FIRST)
    expect(shell.show).not.toHaveBeenCalled()
  })
})

describe('the canvas theme', () => {
  it('follows the appearance preference, and the system when that is system', async () => {
    systemDark = true
    await opened()
    expect(canvas!.theme).toBe('dark')
    cleanup()

    shell.theme = 'light'
    await opened()
    expect(canvas!.theme).toBe('light')
  })
})

describe('pinning a drawing to a card', () => {
  const pinTo = async (title: string): Promise<void> => {
    fireEvent.click(await screen.findByRole('button', { name: 'Pin to card' }))
    fireEvent.click(screen.getByRole('menuitemradio', { name: title }))
  }

  it("pins this drawing, by name, to the card picked from the project's board", async () => {
    await opened()
    await pinTo('Ship it')
    expect(await screen.findByText('Pinned to Ship it.')).toBeTruthy()
    expect(pinnedTo).toHaveBeenCalledWith('excalidraw', NAME, 'card_2')
  })

  it('says why the backend refused', async () => {
    pinRefusal = { code: 'forbidden', message: 'Excalidraw is off for this project' }
    await opened()
    await pinTo('Wire the board')
    expect(await screen.findByText('Excalidraw is off for this project')).toBeTruthy()
    expect(screen.queryByText(/Pinned to/)).toBeNull()
  })

  it('offers no menu when the board has no cards', async () => {
    board = []
    await opened()
    expect(screen.queryByRole('button', { name: 'Pin to card' })).toBeNull()
  })
})

describe('a file the canvas does not open', () => {
  const framed = JSON.stringify({
    type: 'excalidraw',
    version: 2,
    elements: [{ id: 'f', type: 'iframe', customData: { generationData: { status: 'done', html: '<script></script>' } } }],
  })

  it.each([
    ['text that is not a drawing', 'not a drawing'],
    ['a drawing with an iframe element', framed],
  ])('%s is reported and never handed to the canvas, so nothing is saved over it', async (_, text) => {
    files.set(NAME, { text, modified: 5 })
    render(<Drawing tab={tab} name={NAME} />)
    expect(await screen.findByText(/fluxo cannot be opened here, so it is left as it is/)).toBeTruthy()
    expect(screen.queryByTestId('canvas')).toBeNull()
    expect(written).not.toHaveBeenCalled()
  })
})
