import { act, render } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'

import type { Message, Part } from '../gen/bindings'
import { Turn } from './Turn'

vi.mock('./shellStore', () => ({ useShellPick: () => () => undefined }))

const text = (said: string): Part => ({ kind: 'text', text: said, parent: null })
const answer = (parts: Part[]): Message => ({ id: 'a', turnId: null, role: 'assistant', parts, createdAt: 0, streaming: false })

const WHOLE = 'O card do chat fluido da v0.1.36 está fechado e no ar.\n\n**Causa:** a fila.'

describe('an answer kept in pieces', () => {
  it('draws exactly what the whole text draws', () => {
    const pieces = ['O', ' card', ' do chat fluido da', ' v0.1.36 está', ' fechado e no', ' ar.\n\n', '**Causa:** a', ' fila.']
    expect(pieces.join('')).toBe(WHOLE)
    const whole = render(<Turn message={answer([text(WHOLE)])} />).container.innerHTML
    const split = render(<Turn message={answer(pieces.map(text))} />).container.innerHTML
    expect(split).toBe(whole)
    expect(render(<Turn message={answer(pieces.map(text))} />).container.querySelectorAll('.reply')).toHaveLength(1)
  })
})

describe('an answer still arriving', () => {
  it('keeps moving on screen while more of it arrives faster than it is written', () => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'setInterval', 'clearInterval', 'performance', 'Date'] })
    const live = (said: string): Message => ({ ...answer([text(said)]), streaming: true })
    const words = 'word '.repeat(400)
    const { container, rerender } = render(<Turn message={live(words.slice(0, 5))} />)
    /* A piece every 16 ms, as the frames batched for each paint arrive. */
    for (let at = 1; at <= 30; at += 1) {
      rerender(<Turn message={live(words.slice(0, 5 + at * 12))} />)
      act(() => void vi.advanceTimersByTime(16))
    }
    const shown = container.querySelector('.reply')!.textContent!.length
    vi.useRealTimers()
    /* 365 characters arrived in 480 ms; written at its pace, well over 100 are on screen. */
    expect(shown).toBeGreaterThan(100)
  })
})
