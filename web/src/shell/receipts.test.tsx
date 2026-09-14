import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import type { Message } from '../gen/bindings'
import { receiptSubject, Turn } from './Turn'

afterEach(cleanup)

vi.mock('./useShell', () => ({ useShell: () => ({ show: vi.fn() }) }))

const receipt = (allowed: boolean, input: object): Message =>
  ({
    id: 'r',
    turnId: null,
    role: 'system',
    createdAt: 0,
    streaming: false,
    parts: [{ kind: 'receipt', tool: 'Bash', input: JSON.stringify(input), allowed }],
  }) as Message

describe('an answered permission, in the thread', () => {
  it('says it was allowed, and what', () => {
    render(<Turn message={receipt(true, { command: 'cargo test' })} />)
    expect(screen.getByText('Allowed')).toBeTruthy()
    expect(screen.getByText('cargo test')).toBeTruthy()
  })

  it('says it was refused', () => {
    const { container } = render(<Turn message={receipt(false, { file_path: '/work/demo/.env' })} />)
    expect(screen.getByText('Refused')).toBeTruthy()
    expect(container.querySelector('[data-allowed="false"]')).toBeTruthy()
  })

  it('picks the subject a person recognises out of the input', () => {
    expect(receiptSubject(JSON.stringify({ command: 'rm -rf target\necho done' }))).toBe('rm -rf target')
    expect(receiptSubject(JSON.stringify({ file_path: 'a.rs', old_string: 'x' }))).toBe('a.rs')
    expect(receiptSubject('not json')).toBe('')
  })
})

describe("a slash command's own answer", () => {
  it('is shown apart from the reply', () => {
    const message = {
      id: 'm', turnId: 't', role: 'assistant', createdAt: 0, streaming: false,
      parts: [{ kind: 'command', content: 'Compacted. ctrl+o to see full summary' }, { kind: 'text', text: 'Carrying on.', parent: null }],
    } as unknown as Message
    const { container } = render(<Turn message={message} />)
    expect(container.querySelector('.said__cmd')?.textContent).toBe('Compacted. ctrl+o to see full summary')
    expect(screen.getByText('Carrying on.')).toBeTruthy()
  })
})
