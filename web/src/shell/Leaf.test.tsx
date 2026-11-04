import { act, render } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

const fake = vi.hoisted(() => {
  const terms: { disposed: boolean; written: unknown[]; scrolled: ((top: number) => void) | null }[] = []
  const pages: { left: number; bytes: Uint8Array }[] = []
  let asked = 0
  const attaches: { paneId: string; detached: boolean; on: { output: (b: Uint8Array, r: boolean) => void } }[] = []
  class Terminal {
    rows = 24
    cols = 80
    options: Record<string, unknown> = {}
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
    scrollToLine() {}
    dispose() {
      this.disposed = true
    }
  }
  return { terms, attaches, Terminal, pages, asked: () => asked, ask: () => (asked += 1) }
})

vi.mock('@xterm/xterm', () => ({ Terminal: fake.Terminal }))
vi.mock('@xterm/xterm/css/xterm.css', () => ({}))
vi.mock('@xterm/addon-fit', () => ({ FitAddon: class { fit() {} } }))
vi.mock('./terminalAddons', () => ({ gpuWhileShown: () => ({ show() {}, hide() {} }), measureWideCharacters() {} }))
vi.mock('./live', () => ({
  ask: () => Promise.resolve({ data: null }),
  commands: { settingsRead: vi.fn(), sessionRedraw: vi.fn() },
}))
vi.mock('./terminal', () => ({ contrastFor: () => 1, darkNow: () => true, options: () => ({}), palette: () => ({}) }))
vi.mock('./terminalPathLinks', () => ({ linkPaths: () => ({ dispose() {} }) }))
vi.mock('./terminalWheel', () => ({ wheelToTmux: () => ({ dispose() {}, typed() {} }) }))
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
        return fake.pages.shift() ?? null
      },
      detach: () => {
        one.detached = true
      },
    }
  },
}))

import { HIDDEN_DISPOSE_MS, Leaf } from './Leaf'

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
  it('gives back its xterm and its stream once hidden a while, and attaches again when shown', async () => {
    render(<Leaf paneId="leaf_1" projectId="prj_1" />)
    await settle()
    expect(fake.terms).toHaveLength(1)
    expect(fake.attaches).toHaveLength(1)

    vi.useFakeTimers()
    onScreen(false)
    act(() => vi.advanceTimersByTime(HIDDEN_DISPOSE_MS - 1))
    expect(fake.terms[0].disposed).toBe(false)
    act(() => vi.advanceTimersByTime(1))
    expect(fake.terms[0].disposed).toBe(true)
    expect(fake.attaches[0].detached).toBe(true)
    vi.useRealTimers()

    onScreen(true)
    await settle()
    expect(fake.terms).toHaveLength(2)
    expect(fake.attaches).toHaveLength(2)
    expect(fake.attaches[1].detached).toBe(false)
  })

  it('keeps everything through a quick look away', async () => {
    render(<Leaf paneId="leaf_1" projectId="prj_1" />)
    await settle()
    vi.useFakeTimers()
    onScreen(false)
    act(() => vi.advanceTimersByTime(HIDDEN_DISPOSE_MS / 2))
    onScreen(true)
    act(() => vi.advanceTimersByTime(HIDDEN_DISPOSE_MS * 2))
    expect(fake.terms).toHaveLength(1)
    expect(fake.terms[0].disposed).toBe(false)
    expect(fake.attaches[0].detached).toBe(false)
  })

  it('writes the replay straight in and says the pane is drawn', async () => {
    const { container } = render(<Leaf paneId="leaf_1" projectId="prj_1" />)
    await settle()
    const replay = new Uint8Array(600).fill(65)
    act(() => fake.attaches[0].on.output(replay, true))
    expect(fake.terms[0].written).toEqual([replay])
    expect(container.querySelector<HTMLElement>('.termhost')?.dataset.drawn).toBe('true')
  })

  it('scrolled to the top, writes the older page and what it held again, until there is none', async () => {
    render(<Leaf paneId="leaf_1" projectId="prj_1" />)
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
})
