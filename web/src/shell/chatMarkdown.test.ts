import { describe, expect, it } from 'vitest'

import type { Message, Part } from '../gen/bindings'
import { fenced, markdownOf } from './chatMarkdown'
import { blocks } from './markdown'

const message = (id: string, role: Message['role'], parts: Part[]): Message => ({ id, turnId: null, role, parts, createdAt: null, streaming: false })

const CONVERSATION: Message[] = [
  message('m1', 'user', [{ kind: 'text', text: 'Why does the lexer drop tabs?' }]),
  message('m2', 'assistant', [
    { kind: 'text', text: 'Reading it:\n\n```ts\nconst tab = 1' },
    { kind: 'tool_call', id: 'c1', name: 'Read', input: '{"file_path":"src/lexer.ts"}', state: 'failed' },
    { kind: 'tool_result', call_id: 'c1', output: 'tabs are ```skipped```', is_error: true },
    { kind: 'text', text: 'A subagent aside.', parent: 'c1' },
  ]),
  message('m3', 'user', [{ kind: 'text', text: 'Fix it.' }]),
]

describe('a conversation exported as markdown', () => {
  it('keeps only what was said, one heading per message, without tools', () => {
    const md = markdownOf('Lexer tabs', CONVERSATION, 'messages')
    const parsed = blocks(md)
    expect(parsed.filter((one) => one.kind === 'heading').map((one) => one.kind === 'heading' && one.text)).toEqual(['Lexer tabs', 'You', 'Agent', 'You'])
    expect(md).not.toContain('**Read**')
    expect(md).not.toContain('A subagent aside.')
  })

  it('with tools, adds each call and its result as code that ends where it should', () => {
    const md = markdownOf('Lexer tabs', CONVERSATION, 'tools')
    const parsed = blocks(md)
    expect(parsed.filter((one) => one.kind === 'heading').length).toBe(4)
    expect(parsed).toContainEqual({ kind: 'code', language: 'json', text: '{\n  "file_path": "src/lexer.ts"\n}' })
    expect(md).toContain('Failed:')
    expect(parsed.at(-1)).toEqual({ kind: 'paragraph', text: 'Fix it.' })
  })

  it('fences a body with one more backtick than the longest run in it', () => {
    expect(fenced('a ```` b').split('\n')[0]).toBe('`````')
    expect(fenced('plain').split('\n')[0]).toBe('```')
  })
})
