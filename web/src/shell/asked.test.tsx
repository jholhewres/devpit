import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'

import { Asked } from './Asked'

afterEach(cleanup)

const question = { id: 'q1', sessionId: 's1', tool: 'Bash', input: '{"command":"npm test"}', cwd: '/w' }

describe('a question in the chat', () => {
  it('is answered once either way, or always for the rest of the chat', () => {
    const answered = vi.fn()
    render(<Asked questions={[question]} onAnswer={answered} />)
    fireEvent.click(screen.getByText('Always in this chat'))
    fireEvent.click(screen.getByText('Refuse'))
    fireEvent.click(screen.getByText('Allow'))
    expect(answered.mock.calls).toEqual([
      ['q1', 'always'],
      ['q1', 'deny'],
      ['q1', 'allow'],
    ])
  })

  it('says what "always" covers before it is pressed', () => {
    render(<Asked questions={[question]} onAnswer={() => {}} />)
    expect(screen.getByText('Always in this chat').getAttribute('title')).toContain('this exact command')
  })
})
