import { describe, expect, it } from 'vitest'

import type { Message, Part } from '../gen/bindings'
import { landing, stepTo } from './threadKeys'

const message = (id: string, parts: Part[] = [{ kind: 'text', text: id }]): Message => ({ id, turnId: null, role: 'assistant', parts, createdAt: null, streaming: false })
const failed: Part = { kind: 'tool_result', call_id: 'c', output: 'no such file', is_error: true }
const passed: Part = { kind: 'tool_result', call_id: 'c', output: 'ok', is_error: false }
const THREAD = [message('a'), message('b', [passed]), message('c', [failed]), message('d'), message('e', [failed])]

describe('reading a conversation from the keyboard', () => {
  it('steps a message forward and back, and stops at either end', () => {
    expect(stepTo(THREAD, 'b', 'next')).toBe('c')
    expect(stepTo(THREAD, 'b', 'previous')).toBe('a')
    expect(stepTo(THREAD, 'a', 'previous')).toBeNull()
    expect(stepTo(THREAD, 'e', 'next')).toBeNull()
    expect(stepTo(THREAD, null, 'next')).toBe('a')
  })

  it('goes to the next message whose tool failed, past the ones that did not', () => {
    expect(stepTo(THREAD, 'a', 'failed')).toBe('c')
    expect(stepTo(THREAD, 'c', 'failed')).toBe('e')
    expect(stepTo(THREAD, 'e', 'failed')).toBeNull()
  })
})

describe('where a step to a failed tool lands', () => {
  const row = (open: boolean): HTMLElement => {
    const box = document.createElement('div')
    box.innerHTML = `<div class="acts"><button class="acts__h">3 tools</button>${open ? '<div class="arow"><button class="arow__h">Read</button></div><div class="arow" data-failed="true"><button class="arow__h">Bash</button></div>' : ''}</div>`
    return box
  }

  it('is the failed tool when its stack is open', () => {
    expect(landing(row(true), 'failed').textContent).toBe('Bash')
  })

  it('is the stack that holds it when the stack is closed', () => {
    expect(landing(row(false), 'failed').textContent).toBe('3 tools')
  })
})
