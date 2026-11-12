import { act, render } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

const fake = vi.hoisted(() => {
  const terms: { disposed: boolean; cleared: number; written: unknown[]; scrolled: ((top: number) => void) | null; write: (data: unknown, done?: () => void) => void }[] = []
  const pages: { left: number; bytes: Uint8Array }[] = []
  let asked = 0
  const attaches: { paneId: string; detached: boolean; on: { output: (b: Uint8Array, r: boolean) => void } }[] = []
  class Terminal {
    rows = 24
    cols = 80
    options: Record<string, unknown> = { scrollback: 100_000 }
    buffer = { active: { type: 'normal', length: 24 } }
    disposed = false
    written: unknown[] = []
    constructor() {
      terms.push(this)
    }
    loadAddon() {}
    open() {}
    write(data: unknown, done?: () => void) {
      this.written.push(data)
      /* A reset scrolls to the top, as xterm's does. */
      if (data === '\x1bc') this.scrolled?.(0)
      done?.()
    }
    onData() {
      return { dispose() {} }
    }
    scrolled: ((top: number) => void) | null = null
    onScroll(run: (top: number) => void) {
      this.scrolled = run
      return { dispose() {} }
    }
    refresh() {}
    resize() {}
    cleared = 0
    reset() {
      this.cleared += 1
    }
    scrollToLine() {}
    dispose() {
      this.disposed = true
    }
  }
  return {
    terms,
    attaches,
    Terminal,
    pages,
    asked: () => asked,
    ask: () => (asked += 1),
    waiting: { current: null as Promise<void> | null },
    replaying: { current: (() => false) as () => boolean },
    replayedClear: { current: (() => {}) as () => void },
  }
})

vi.mock('@xterm/xterm', () => ({ Terminal: fake.Terminal }))
vi.mock('@xterm/xterm/css/xterm.css', () => ({}))
vi.mock('@xterm/addon-fit', () => ({ FitAddon: class { fit() {} } }))
vi.mock('./terminalAddons', () => ({ gpuWhileShown: () => ({ show() {}, hide() {}, away() {} }), measureWideCharacters() {} }))
vi.mock('./live', () => ({
  ask: () => Promise.resolve({ data: null }),
  commands: { settingsRead: vi.fn(), sessionRedraw: vi.fn() },
}))
vi.mock('./terminal', () => ({ contrastFor: () => 1, darkNow: () => true, options: () => ({}), palette: () => ({}) }))
vi.mock('./terminalPathLinks', () => ({ linkPaths: () => ({ dispose() {} }) }))
vi.mock('./terminalFollow', () => ({
  followAfterClear: (_terminal: unknown, replaying: () => boolean, replayedClear: () => void) => {
    fake.replaying.current = replaying
    fake.replayedClear.current = replayedClear
    return { dispose() {} }
  },
}))
vi.mock('./terminalWheel', () => ({ wheelFor: () => ({ dispose() {}, typed() {} }) }))
vi.mock('./terminalIme', () => ({ guardComposition: () => () => {} }))
vi.mock('./terminalPaste', () => ({ picturesAsPaths: () => () => {} }))
vi.mock('./useMarks', () => ({ useMarks() {} }))
vi.mock('./shellStore', () => ({ useShellPick: () => ({ show: vi.fn(), root: null }) }))
vi.mock('./fileDrop', () => ({ shellWord: (s: string) => s, useFileDrop: () => false }))
vi.mock('./attach', () => ({
  attachPane: async (_project: string, paneId: string, _size: unknown, on: never) => {
    const one = { paneId, detached: false, on }
    fake.attaches.push(one)
    return {
      write() {},
      resize: async () => null,
      ack() {},
      older: async () => {
        fake.ask()
        await fake.waiting.current
        return fake.pages.shift() ?? null
      },
      detach: () => {
        one.detached = true
      },
    }
  },
}))

import { drawFrom, HIDDEN_RELEASE_MS, Leaf, REPLAY_FIRST } from './Leaf'

let seen: ((entries: { isIntersecting: boolean }[]) => void)[] = []
class Observer {
  constructor(run: (entries: { isIntersecting: boolean }[]) => void) {
    seen.push(run)
  }
  observe() {}
  disconnect() {}
}

const settle = () => act(async () => await new Promise((done) => setTimeout(done, 0)))
const onScreen = (yes: boolean) => act(() => seen.forEach((run) => run([{ isIntersecting: yes }])))

beforeEach(() => {
  seen = []
  fake.terms.length = 0
  fake.attaches.length = 0
  fake.waiting.current = null
  vi.stubGlobal('IntersectionObserver', Observer)
  vi.stubGlobal('ResizeObserver', class { observe() {} disconnect() {} })
  vi.stubGlobal('requestAnimationFrame', () => 0)
  vi.stubGlobal('cancelAnimationFrame', () => {})
})
afterEach(() => {
  vi.useRealTimers()
  vi.unstubAllGlobals()
})

describe('a terminal off screen', () => {
  it('lets its stream go at once and its rows once hidden a while, and attaches again when shown', async () => {
    render(<Leaf paneId="leaf_1" projectId="prj_1" />)
    onScreen(true)
    await settle()
    expect(fake.terms).toHaveLength(1)
    expect(fake.attaches).toHaveLength(1)

    vi.useFakeTimers()
    onScreen(false)
    expect(fake.attaches[0].detached).toBe(true)
    act(() => vi.advanceTimersByTime(HIDDEN_RELEASE_MS - 1))
    expect(fake.terms[0].cleared).toBe(0)
    act(() => vi.advanceTimersByTime(1))
    expect(fake.terms[0].cleared).toBe(1)
    vi.useRealTimers()

    onScreen(true)
    await settle()
    expect(fake.terms).toHaveLength(1)
    expect(fake.terms[0].disposed).toBe(false)
    expect(fake.attaches).toHaveLength(2)
    expect(fake.attaches[1].detached).toBe(false)
  })

  it('keeps its xterm through a quick look away, and draws the new attach over a reset screen', async () => {
    render(<Leaf paneId="leaf_1" projectId="prj_1" />)
    onScreen(true)
    await settle()
    act(() => fake.attaches[0].on.output(Uint8Array.from([65]), true))
    vi.useFakeTimers()
    onScreen(false)
    act(() => vi.advanceTimersByTime(HIDDEN_RELEASE_MS / 2))
    onScreen(true)
    vi.useRealTimers()
    await settle()

    expect(fake.terms).toHaveLength(1)
    expect(fake.terms[0].disposed).toBe(false)
    expect(fake.attaches).toHaveLength(2)
    expect(fake.attaches[0].detached).toBe(true)
    const late = Uint8Array.from([88])
    const replay = Uint8Array.from([66])
    act(() => fake.attaches[0].on.output(late, false))
    act(() => fake.attaches[1].on.output(replay, true))
    expect(fake.terms[0].written).toEqual([Uint8Array.from([65]), '\x1bc', new Uint8Array(), replay])
    expect(fake.asked()).toBe(0)
  })

  it('attaches nothing while it was never on screen', async () => {
    render(<Leaf paneId="leaf_1" projectId="prj_1" />)
    await settle()
    expect(fake.terms).toHaveLength(1)
    expect(fake.attaches).toHaveLength(0)
  })

  it('writes the replay straight in and says the pane is drawn', async () => {
    const { container } = render(<Leaf paneId="leaf_1" projectId="prj_1" />)
    onScreen(true)
    await settle()
    const replay = new Uint8Array(600).fill(65)
    act(() => fake.attaches[0].on.output(replay, true))
    expect(fake.terms[0].written).toEqual([replay])
    expect(container.querySelector<HTMLElement>('.termhost')?.dataset.drawn).toBe('true')
  })

  it('draws only the end of a long replay, and the rest once scrolled to, without asking for it', async () => {
    render(<Leaf paneId="leaf_1" projectId="prj_1" />)
    onScreen(true)
    await settle()
    const term = fake.terms[0]
    const replay = new TextEncoder().encode('a line of history\r\n'.repeat(Math.ceil((REPLAY_FIRST * 2) / 19)))
    act(() => fake.attaches[0].on.output(replay, true))
    expect(term.written).toEqual([replay.subarray(drawFrom(replay))])
    expect((term.written[0] as Uint8Array).length).toBeLessThanOrEqual(REPLAY_FIRST)

    await act(async () => term.scrolled?.(0))
    await settle()
    expect(term.written.slice(1)).toEqual(['\x1bc', replay, new Uint8Array()])
    expect(fake.asked()).toBe(0)
  })

  it('scrolled to the top, writes the older page and what it held again, until there is none', async () => {
    render(<Leaf paneId="leaf_1" projectId="prj_1" />)
    onScreen(true)
    await settle()
    const term = fake.terms[0]
    act(() => fake.attaches[0].on.output(Uint8Array.from([66]), true))
    fake.pages.push({ left: 0, bytes: Uint8Array.from([65]) })
    await act(async () => term.scrolled?.(0))
    await settle()
    expect(term.written.slice(1)).toEqual(['\x1bc', Uint8Array.from([65]), Uint8Array.from([66]), new Uint8Array()])
    await act(async () => term.scrolled?.(0))
    await settle()
    expect(fake.asked()).toBe(1)
  })

  it('counts the bytes it writes again around a page as a replay, so their clears do not move the reader', async () => {
    render(<Leaf paneId="leaf_1" projectId="prj_1" />)
    onScreen(true)
    await settle()
    const term = fake.terms[0]
    act(() => fake.attaches[0].on.output(Uint8Array.from([66]), true))
    fake.pages.push({ left: 0, bytes: Uint8Array.from([65]) })
    expect(fake.replaying.current()).toBe(false)
    const seen: boolean[] = []
    const write = term.write.bind(term)
    term.write = (data: unknown, done?: () => void) => {
      seen.push(fake.replaying.current())
      write(data, done)
    }
    await act(async () => term.scrolled?.(0))
    await settle()
    expect(seen.length).toBeGreaterThan(0)
    expect(seen.every(Boolean)).toBe(true)
    expect(fake.replaying.current()).toBe(false)
  })

  it('asks no further page once a clear in what it wrote again left the page no rows', async () => {
    render(<Leaf paneId="leaf_1" projectId="prj_1" />)
    onScreen(true)
    await settle()
    const term = fake.terms[0]
    act(() => fake.attaches[0].on.output(Uint8Array.from([66]), true))
    fake.pages.push({ left: 5, bytes: Uint8Array.from([65]) }, { left: 4, bytes: Uint8Array.from([67]) })
    const write = term.write.bind(term)
    term.write = (data: unknown, done?: () => void) => {
      /* The held bytes after the page clear the scrollback, as a redraw does. */
      if (data instanceof Uint8Array && data[0] === 66) fake.replayedClear.current()
      write(data, done)
    }
    const before = fake.asked()
    await act(async () => term.scrolled?.(0))
    await settle()
    await act(async () => term.scrolled?.(0))
    await settle()
    expect(fake.asked() - before).toBe(1)
    fake.pages.length = 0
  })

  /* A page redrawn in place (a progress bar) draws no rows, yet older rows
     still sit before it. The view stays at the top, where a wheel moves
     nothing and no scroll would ask again: the next page is asked at once. */
  it('asks the next page at once after one that drew no rows while nothing cleared', async () => {
    render(<Leaf paneId="leaf_1" projectId="prj_1" />)
    onScreen(true)
    await settle()
    const term = fake.terms[0]
    act(() => fake.attaches[0].on.output(Uint8Array.from([66]), true))
    fake.pages.push({ left: 5, bytes: Uint8Array.from([65]) }, { left: 0, bytes: Uint8Array.from([67]) })
    const before = fake.asked()
    await act(async () => term.scrolled?.(0))
    await settle()
    expect(fake.asked() - before).toBe(2)
    expect(term.written.slice(-4)).toEqual([Uint8Array.from([67]), Uint8Array.from([65]), Uint8Array.from([66]), new Uint8Array()])
    fake.pages.length = 0
  })

  it('takes a clear that arrives while an older page is on its way as the program’s own', async () => {
    render(<Leaf paneId="leaf_1" projectId="prj_1" />)
    onScreen(true)
    await settle()
    const term = fake.terms[0]
    act(() => fake.attaches[0].on.output(Uint8Array.from([66]), true))
    let arrive = (): void => {}
    fake.waiting.current = new Promise<void>((done) => (arrive = done))
    fake.pages.push({ left: 0, bytes: Uint8Array.from([65]) })
    const before = fake.asked()
    await act(async () => term.scrolled?.(0))
    expect(fake.asked() - before).toBe(1)
    expect(fake.replaying.current()).toBe(false)
    fake.waiting.current = null
    arrive()
    await settle()
  })
})
