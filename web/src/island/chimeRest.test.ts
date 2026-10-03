import { afterEach, describe, expect, it, vi } from 'vitest'

import { play, REST_AFTER_MS } from './chime'

/* A context that only says what was done to it. */
class Context {
  static made = 0
  static last: Context | null = null
  state: AudioContextState = 'running'
  currentTime = 0
  destination = {}
  suspended = 0
  constructor() {
    Context.made += 1
    Context.last = this
  }
  createOscillator() {
    return { type: 'sine', frequency: { value: 0 }, connect: (to: unknown) => to, start: () => {}, stop: () => {} }
  }
  createGain() {
    const node = { gain: { setValueAtTime: () => {}, exponentialRampToValueAtTime: () => {} }, connect: (to: unknown) => to }
    return node
  }
  suspend() {
    this.suspended += 1
    this.state = 'suspended'
    return Promise.resolve()
  }
  resume() {
    this.state = 'running'
    return Promise.resolve()
  }
}

afterEach(() => {
  vi.useRealTimers()
  vi.unstubAllGlobals()
})

describe('the island audio', () => {
  it('lets the audio go a moment after the last cue, and wakes it for the next', () => {
    vi.useFakeTimers()
    vi.stubGlobal('AudioContext', Context)
    play('done')
    const made = Context.made
    const audio = Context.last!
    expect(audio.suspended).toBe(0)
    vi.advanceTimersByTime(REST_AFTER_MS + 1000)
    expect(audio.suspended).toBe(1)
    expect(audio.state).toBe('suspended')

    play('asking')
    expect(audio.state).toBe('running')
    vi.advanceTimersByTime(REST_AFTER_MS + 1000)
    expect(audio.suspended).toBe(2)
    expect(Context.made).toBe(made)
  })

  /* WebKit refuses to resume audio before the person has touched the window;
     that refusal is no error worth a report. */
  it('lets a refused wake-up go without a loose rejection', async () => {
    vi.stubGlobal('AudioContext', Context)
    play('done')
    const audio = Context.last!
    audio.state = 'suspended'
    audio.resume = () => Promise.reject(new Error('not allowed to start'))
    const loose = vi.fn()
    process.on('unhandledRejection', loose)
    try {
      play('done')
      await new Promise((settled) => setTimeout(settled, 0))
      expect(loose).not.toHaveBeenCalled()
    } finally {
      process.off('unhandledRejection', loose)
    }
  })
})
