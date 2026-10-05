import { render } from '@testing-library/react'
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
