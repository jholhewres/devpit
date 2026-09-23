import { afterEach, describe, expect, it, vi } from 'vitest'

const scrolled = vi.fn()
vi.mock('./live', () => ({
  ask: (run: () => unknown) => Promise.resolve(run()).then((data) => ({ data, error: null })),
  commands: { sessionScroll: (...args: unknown[]) => scrolled(...args) },
}))

import { wheelToTmux } from './terminalWheel'

afterEach(() => {
  scrolled.mockReset()
  vi.useRealTimers()
})

function rig() {
  let handler: (event: WheelEvent) => boolean = () => true
  const terminal = { rows: 10, attachCustomWheelEventHandler: (one: typeof handler) => (handler = one) }
  const box = { clientHeight: 200 } as HTMLElement
  const wheel = wheelToTmux(terminal as never, box, 'p1', 'leaf')
  const spin = (deltaY: number): boolean =>
    handler({ deltaX: 0, deltaY, deltaMode: 0, preventDefault: () => {} } as unknown as WheelEvent)
  return { wheel, spin }
}

describe('the wheel over a tmux terminal', () => {
  it('asks tmux for whole lines, a flick at a time, and never lets xterm send arrows', () => {
    vi.useFakeTimers()
    const { spin } = rig()
    expect(spin(-30)).toBe(false)
    expect(spin(-30)).toBe(false)
    vi.advanceTimersByTime(50)
    // 60px over 20px rows: three lines up.
    expect(scrolled).toHaveBeenCalledWith('p1', 'leaf', -3)
  })

  it('goes back to the live screen before a key typed after scrolling up', async () => {
    vi.useFakeTimers()
    const { wheel, spin } = rig()
    spin(-40)
    vi.advanceTimersByTime(50)
    vi.useRealTimers()
    const write = vi.fn()
    wheel.typed('a', write)
    expect(scrolled).toHaveBeenLastCalledWith('p1', 'leaf', 0)
    await new Promise((done) => setTimeout(done, 0))
    expect(write).toHaveBeenCalledWith('a')
    wheel.typed('b', write)
    expect(write).toHaveBeenLastCalledWith('b')
  })
})
